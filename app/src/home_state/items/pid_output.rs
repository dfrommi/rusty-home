use crate::{
    automation::{HeatingZone, Thermostat},
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Interpolator, LinearOrLastSeenInterpolator},
        },
        unit::{DegreeCelsius, Percent},
    },
    home_state::{
        HeatingMode, TargetHeatingMode, Temperature,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum PidOutput {
    Thermostat(Thermostat),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PidResult {
    p_term: Percent,
    i_term: Percent,
    d_term: Percent,
}

impl PidResult {
    pub fn p(&self) -> Percent {
        self.p_term
    }

    pub fn i(&self) -> Percent {
        self.i_term
    }

    pub fn d(&self) -> Percent {
        self.d_term
    }

    pub fn total(&self) -> Percent {
        self.p_term + self.i_term + self.d_term
    }
}

pub struct PidOutputStateProvider;

impl DerivedStateProvider<PidOutput, PidResult> for PidOutputStateProvider {
    fn calculate_current(&self, id: PidOutput, ctx: &StateCalculationContext) -> Option<PidResult> {
        let (thermostat, mode) = match id {
            PidOutput::Thermostat(t) => (t, TargetHeatingMode::from_thermostat(t)),
        };

        let temperature_id = match thermostat {
            Thermostat::LivingRoomBig => Temperature::LivingRoom,
            Thermostat::LivingRoomSmall => Temperature::LivingRoom,
            Thermostat::Bedroom => Temperature::Bedroom,
            Thermostat::Kitchen => Temperature::Kitchen,
            Thermostat::RoomOfRequirements => Temperature::RoomOfRequirements,
            Thermostat::Bathroom => Temperature::Bathroom,
        };

        let heating_zone = HeatingZone::for_thermostat(&thermostat);

        let mode = ctx.get(mode)?;
        let setpoint = DataPoint::new(heating_zone.setpoint_for_mode(&mode.value), mode.timestamp);
        let temperatures = ctx.all_since(temperature_id, t!(3 hours ago))?;

        let params = get_pid_config_for_mode(&mode.value, &thermostat);

        calculate_pid(&params, setpoint, &temperatures)
    }
}

fn get_pid_config_for_mode(mode: &HeatingMode, thermostat: &Thermostat) -> PidConfig {
    match (mode, thermostat) {
        (HeatingMode::Ventilation, _) => PidConfig::new(0.0, 0.0, 0.0, None),
        (HeatingMode::EnergySaving, Thermostat::Kitchen) => PidConfig::new(15.0, 40.0, 0.0, None),
        (_, _) => PidConfig::time_based_gains(25.0, t!(30 minutes), Duration::zero(), None),
    }
}

struct PidConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,

    //slow down adjustment when close to target
    pub deadband: Option<DeadbandConfig>,
}

impl PidConfig {
    fn new(kp: f64, ki: f64, kd: f64, deadband: Option<DeadbandConfig>) -> Self {
        Self { kp, ki, kd, deadband }
    }

    //create PID config from time-based parameters
    //
    //ki=kp/ti -> time when integral alone fixes the error, if the error remains constant
    //small: aggressive but over-shoots
    //large: slow to react
    //
    //kd=kp*td -> how much does the calculation look ahead
    //large: strong damping, react early to changes (less overshoot)
    //small: little damping, more overshoot, but faster reaction
    fn time_based_gains(kp: f64, ti: Duration, td: Duration, deadband: Option<DeadbandConfig>) -> Self {
        let ki = if ti > Duration::zero() {
            kp / ti.as_hours_f64()
        } else {
            0.0
        };
        let kd = kp * td.as_hours_f64();

        Self::new(kp, ki, kd, deadband)
    }
}

struct DeadbandConfig {
    threshhold_above: DegreeCelsius,
    threshhold_below: DegreeCelsius,
    ki_multiplier: f64,
}

impl DeadbandConfig {
    fn is_in_deadband(&self, error: DegreeCelsius) -> bool {
        if error.0 > 0.0 {
            error <= self.threshhold_above
        } else {
            error >= -self.threshhold_below
        }
    }
}

fn calculate_pid(
    params: &PidConfig,
    setpoint: DataPoint<DegreeCelsius>,
    temperatures: &DataFrame<DegreeCelsius>,
) -> Option<PidResult> {
    let (mut kp, mut kd) = (params.kp, params.kd);

    let range = DateTimeRange::since(t!(60 minutes ago).max(setpoint.timestamp));
    let error_df = temperatures.map(|dp| setpoint.value - dp.value).retain_range(
        &range,
        LinearOrLastSeenInterpolator,
        LinearOrLastSeenInterpolator,
    );

    //P-controller
    let error = error_df.last()?.value;

    //reduce P and D gains when in deadband
    if let Some(deadband) = &params.deadband
        && deadband.is_in_deadband(error)
    {
        kp = 0.0;
        kd = 0.0;
    }

    //I-controller
    let mut integral_h = 0.0;
    let mut error_current_next = error_df.current_and_next().into_iter();
    while let Some((prev, Some(next))) = error_current_next.next() {
        //if in deadband, slow down adjustments
        let ki = if let Some(deadband) = &params.deadband
            && deadband.is_in_deadband(prev.value)
            && deadband.is_in_deadband(next.value)
        {
            deadband.ki_multiplier * params.ki
        } else {
            params.ki
        };

        let value = LinearOrLastSeenInterpolator
            .interpolate(DateTime::midpoint(&prev.timestamp, &next.timestamp), prev, next)
            .unwrap_or(prev.value);
        let dt_h = next.timestamp.elapsed_since(prev.timestamp).as_hours_f64().max(1e-6);

        integral_h += ki * value.0 * dt_h;
    }

    //clamp integral to avoid windup
    integral_h = integral_h.clamp(-100.0, 100.0);

    //D-controller
    let derivative_h = error_df
        .last2()
        .map(|(prev, last)| {
            let dt_h = last.timestamp.elapsed_since(prev.timestamp).as_hours_f64().max(1e-6);
            (last.value.0 - prev.value.0) / dt_h
        })
        .unwrap_or(0.0);

    Some(PidResult {
        p_term: Percent(kp * error.0),
        i_term: Percent(integral_h),
        d_term: Percent(-kd * derivative_h),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeseries::DataPoint;

    macro_rules! df {
        ( $( $time:literal minutes ago => $value:expr ),* $(,)? ) => {
            {
                let dps  = vec![
                $(
                    DataPoint::new(DegreeCelsius($value), crate::core::time::DateTime::now() - crate::core::time::Duration::minutes($time)),
                )*
                ];
                DataFrame::new(dps)
            }
        };
    }

    #[test]
    fn test_pid_calculation() {
        let setpoint = DataPoint::new(DegreeCelsius(19.8), t!(35 minutes ago));
        let config = PidConfig::time_based_gains(25.0, t!(30 minutes), Duration::zero(), None);

        println!("PID Config: kp={}, ki={}, kd={}", config.kp, config.ki, config.kd);

        let temperatures = df![
            30 minutes ago => 19.0,
            20 minutes ago => 19.1,
            10 minutes ago => 19.2,
        ];

        let output = calculate_pid(&config, setpoint, &temperatures).unwrap();

        println!("PID output: {:?}", output);

        assert!(output.total() > Percent(30.0));
        assert!(output.total() < Percent(40.0));
    }

    #[test]
    fn test_pid_with_deadband() {
        let setpoint = DataPoint::new(DegreeCelsius(19.7), t!(35 minutes ago));
        let config = PidConfig::time_based_gains(
            25.0,
            t!(30 minutes),
            Duration::zero(),
            Some(DeadbandConfig {
                threshhold_above: DegreeCelsius(0.5),
                threshhold_below: DegreeCelsius(0.5),
                ki_multiplier: 0.2,
            }),
        );

        println!("PID Config: kp={}, ki={}, kd={}", config.kp, config.ki, config.kd);

        let temperatures = df![
            30 minutes ago => 19.0,
            20 minutes ago => 19.1,
            10 minutes ago => 19.2,
        ];

        let output = calculate_pid(&config, setpoint, &temperatures).unwrap();

        println!("PID output: {:?}", output);

        assert!(output.total() > Percent(10.0));
        assert!(output.total() < Percent(20.0));
    }
}
