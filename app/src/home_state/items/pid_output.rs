use crate::{
    automation::{HeatingZone, Thermostat},
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Interpolator, LastSeenInterpolator, LinearOrLastSeenInterpolator},
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

        let lookback_start = t!(3 hours ago);

        let modes = ctx.all_since(mode, lookback_start)?;
        let temperatures = ctx.all_since(temperature_id, lookback_start)?;
        let setpoints = ctx.all_since(thermostat.set_point(), lookback_start)?;

        let params = modes.map(|mode_dp| get_pid_config_for_mode(&mode_dp.value, &thermostat));

        let errors = DataFrame::by_reducing2(
            (&temperatures, LinearOrLastSeenInterpolator),
            (&setpoints, LastSeenInterpolator),
            |temp_dp, setpoint_dp| setpoint_dp.value - temp_dp.value,
        )
        .retain_range(
            //truncate strictly to range
            &DateTimeRange::since(t!(1 hours ago)),
            LinearOrLastSeenInterpolator,
            LastSeenInterpolator,
        );

        calculate_pid(params, errors)
    }
}

fn get_pid_config_for_mode(mode: &HeatingMode, thermostat: &Thermostat) -> PidConfig {
    match (mode, thermostat) {
        (HeatingMode::Manual(_, _), _) => PidConfig::new(30.0, 10.0, 0.0),
        (HeatingMode::Ventilation, _) => PidConfig::new(0.0, 0.0, 0.0),
        (HeatingMode::PostVentilation, _) => PidConfig::new(0.0, 0.0, 0.0),
        (_, Thermostat::Kitchen) => PidConfig::new(15.0, 5.0, 0.0),
        (_, _) => PidConfig::time_based_gains(25.0, t!(30 minutes), Duration::zero()),
    }
}

#[derive(Debug, Clone)]
struct PidConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

impl PidConfig {
    fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self { kp, ki, kd }
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
    fn time_based_gains(kp: f64, ti: Duration, td: Duration) -> Self {
        let ki = if ti > Duration::zero() {
            kp / ti.as_hours_f64()
        } else {
            0.0
        };
        let kd = kp * td.as_hours_f64();

        Self::new(kp, ki, kd)
    }
}

fn calculate_pid(params: DataFrame<PidConfig>, errors: DataFrame<DegreeCelsius>) -> Option<PidResult> {
    let current_params = params.last()?.value.clone();

    //P-controller
    let error = current_params.kp * errors.last()?.value;

    //D-controller
    let derivative_h = -current_params.kd
        * errors
            .last2()
            .map(|(prev, last)| {
                let dt_h = last.timestamp.elapsed_since(prev.timestamp).as_hours_f64().max(1e-6);
                (last.value.0 - prev.value.0) / dt_h
            })
            .unwrap_or(0.0);

    //I-controller
    let mut integral_h = 0.0;

    for (prev, next) in errors.current_and_next() {
        let next = match next {
            Some(n) => n,
            None => &DataPoint::new(prev.value, t!(now)),
        };

        let section_mid_time = DateTime::midpoint(&prev.timestamp, &next.timestamp);
        let section_length = next.timestamp.elapsed_since(prev.timestamp).as_hours_f64().max(1e-6);

        let value = LinearOrLastSeenInterpolator
            .interpolate(section_mid_time, prev, next)
            .unwrap_or(prev.value);

        let ki = params.prev_or_at(section_mid_time).map(|p| p.value.ki).unwrap_or(0.0);

        integral_h += ki * value.0 * section_length;
    }

    //clamp integral to avoid windup
    integral_h = integral_h.clamp(-100.0, 100.0);

    Some(PidResult {
        p_term: Percent(error.0),
        i_term: Percent(integral_h),
        d_term: Percent(derivative_h),
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
        let config = PidConfig::time_based_gains(25.0, t!(30 minutes), Duration::zero());
        println!("PID Config: kp={}, ki={}, kd={}", config.kp, config.ki, config.kd);

        let configs = DataFrame::new(vec![DataPoint::new(config, t!(3 hours ago))]);

        let errors = df![
            30 minutes ago => 0.8,
            20 minutes ago => 0.7,
            10 minutes ago => 0.6,
        ];

        let output = calculate_pid(configs, errors).unwrap();

        println!("PID output: {:?}", output);

        assert!(output.total() > Percent(30.0));
        assert!(output.total() < Percent(40.0));
    }
}
