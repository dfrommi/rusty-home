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
pub enum TargetHeatingDemand {
    Thermostat(Thermostat),
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<TargetHeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: TargetHeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        let (thermostat, mode) = match id {
            TargetHeatingDemand::Thermostat(t) => (t, TargetHeatingMode::from_thermostat(t)),
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

        let setpoint = heating_zone.setpoint_for_mode(&mode.value);
        let (current_demand, current_demand_elapsed) = ctx
            .get(thermostat.heating_demand())
            .map(|dp| (dp.value, dp.timestamp.elapsed()))?;
        let params = get_pid_params_for_mode(&thermostat, &mode.value);

        let mut pid_demand = match mode.value {
            HeatingMode::Ventilation => Percent(0.0),

            _ => {
                let start_at = mode.timestamp;

                let temperatures = ctx.all_since(temperature_id, start_at);

                let output = if let Some(temperatures) = temperatures
                    && !temperatures.is_empty()
                {
                    calculate_output(params.clone(), mode, current_demand, setpoint, temperatures)
                } else {
                    None
                };

                output.unwrap_or(current_demand)
            }
        };

        //TODO
        //deadband

        //stay on 0 when already at 0 until significant demand (with hysteresis)
        pid_demand = if (current_demand == Percent(0.0) && pid_demand < Percent(3.0))
            || (current_demand > Percent(0.0) && pid_demand < Percent(1.0))
        {
            Percent(0.0)
        } else {
            Percent(pid_demand.0.clamp(0.0, params.max_relative_output.0))
        };

        //If demand is zero, return early, no need to further process
        if pid_demand == Percent(0.0) {
            return Some(pid_demand);
        }

        //scale into range that produces heat
        let range = Percent(100.0) - params.min_output_for_heat;
        let scaled_demand = (params.min_output_for_heat + (pid_demand.factor() * range))
            .round()
            .clamp();

        //In case of weird calculation results, should never happen, but still important to detect
        if scaled_demand.0.is_nan() {
            tracing::error!("Calculated NaN heating demand for {:?}, reverting to current demand", id);
            return Some(current_demand);
        }

        //Only change output if significant change or after time
        let change = (scaled_demand - current_demand).abs();
        if change < Percent(5.0) && current_demand_elapsed < t!(15 minutes) {
            return Some(current_demand);
        }

        Some(scaled_demand.round())
    }
}

#[derive(Debug, Clone)]
struct PidParams {
    kp: f64,
    ki: f64,
    kd: f64,
    min_output_for_heat: Percent,
    max_relative_output: Percent,
    deadband: DegreeCelsius,
}

fn get_pid_params_for_mode(thermostat: &Thermostat, mode: &HeatingMode) -> PidParams {
    let min_output_with_heating_effect = match thermostat {
        Thermostat::LivingRoomBig => Percent(10.0),
        Thermostat::LivingRoomSmall => Percent(20.0),
        Thermostat::Bedroom => Percent(20.0),
        Thermostat::Kitchen => Percent(20.0),
        Thermostat::RoomOfRequirements => Percent(20.0),
        Thermostat::Bathroom => Percent(20.0),
    };

    match mode {
        HeatingMode::EnergySaving => PidParams {
            kp: 35.0,
            ki: 8.0,
            kd: 4.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(80.0),
            deadband: DegreeCelsius(0.3),
        },
        HeatingMode::Comfort => PidParams {
            kp: 50.0,
            ki: 10.0,
            kd: 5.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(100.0),
            deadband: DegreeCelsius(0.2),
        },
        HeatingMode::Sleep => PidParams {
            kp: 20.0,
            ki: 4.0,
            kd: 2.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(60.0),
            deadband: DegreeCelsius(0.4),
        },
        HeatingMode::Away => PidParams {
            kp: 15.0,
            ki: 3.0,
            kd: 2.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(40.0),
            deadband: DegreeCelsius(0.5),
        },
        HeatingMode::Ventilation => PidParams {
            kp: 0.0,
            ki: 0.0,
            kd: 0.0,
            min_output_for_heat: Percent(0.0),
            max_relative_output: Percent(0.0),
            deadband: DegreeCelsius(0.0),
        },
        HeatingMode::PostVentilation => PidParams {
            kp: 30.0,
            ki: 6.0,
            kd: 3.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(25.0),
            deadband: DegreeCelsius(0.25),
        },
        HeatingMode::Manual(_, _) => PidParams {
            kp: 70.0,
            ki: 15.0,
            kd: 6.0,
            min_output_for_heat: min_output_with_heating_effect,
            max_relative_output: Percent(100.0),
            deadband: DegreeCelsius(0.1),
        },
    }
}

fn calculate_output(
    params: PidParams,
    mode: DataPoint<HeatingMode>,
    current_demand: Percent,
    setpoint: DegreeCelsius,
    temperatures: DataFrame<DegreeCelsius>,
) -> Option<Percent> {
    let start_at = mode.timestamp;

    let current_temperature = temperatures.last()?.value;

    let error = setpoint - current_temperature;

    if matches!(mode.value, HeatingMode::Manual(_, _)) && error >= DegreeCelsius(1.0) {
        //One degree is 50% opening
        return Percent(error.0 * 50.0).clamp().into();
    }

    let output = calculate_pid(&params, current_demand, setpoint, &temperatures, start_at, t!(30 seconds))
        .unwrap_or(current_demand)
        .clamp();

    Some(output)
}

fn calculate_pid(
    params: &PidParams,
    current_heating_demand: Percent, // actual or last-commanded TRV opening
    setpoint: DegreeCelsius,
    temperatures: &DataFrame<DegreeCelsius>,
    start_at: DateTime,
    step: Duration,
) -> Option<Percent> {
    // Limit how far back we integrate.
    // PID-relevant thermal memory is typically tens of minutes, not hours.
    let history_start = t!(60 minutes ago).max(start_at);
    let range = DateTimeRange::since(history_start);

    let dt_h = step.as_hours_f64().max(1e-6);
    let dt_s = dt_h * 3600.0;

    // --- Leaky integral time constant (seconds).
    // Old errors gradually lose influence.
    let tau_i_s: f64 = 30.0 * 60.0; // 30 minutes
    let leak = (-dt_s / tau_i_s).exp();

    let mut integral = 0.0;
    let mut prev_temp: Option<f64> = None;
    let mut output = 0.0;

    // Integral cap so Ki * I alone cannot exceed max_output
    let integral_cap = if params.ki.abs() > 1e-9 {
        100.0 / params.ki.abs()
    } else {
        0.0
    };

    let mut sample_count = 0;

    for dt in range.step_by(step) {
        let temperature = match LinearOrLastSeenInterpolator.interpolate_df(dt, temperatures) {
            Some(v) => v,
            None => {
                prev_temp = None;
                continue;
            }
        };

        let error = (setpoint - temperature).0;

        // --- Derivative on measurement (avoids derivative kick on setpoint changes)
        let dtemp = prev_temp.map(|prev| (temperature.0 - prev) / dt_h).unwrap_or(0.0);

        let p_term = params.kp * error;
        let d_term = -params.kd * dtemp;

        // --- Anti-windup gating using actual heating demand
        let pushing_high = current_heating_demand.0 >= params.max_relative_output.0 && error > 0.0;
        let pushing_low = current_heating_demand.0 <= params.min_output_for_heat.0 && error < 0.0;

        let outside_deadband = error.abs() > params.deadband.0;

        if outside_deadband && !pushing_high && !pushing_low {
            // Normal integration with leak
            integral = integral * leak + error * dt_h;
        } else {
            // Let old integral decay, but do not build new windup
            integral *= leak;
        }

        integral = integral.clamp(-integral_cap, integral_cap);

        let i_term = params.ki * integral;

        output = p_term + i_term + d_term;

        prev_temp = Some(temperature.0);
        sample_count += 1;
    }

    if sample_count < 2 {
        return None;
    }

    Some(Percent(output))
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
        let mode = DataPoint::new(HeatingMode::Comfort, t!(30 minutes ago));
        let setpoint = DegreeCelsius(20.0);
        let current_demand = Percent(50.0);

        let temperatures = df![
            30 minutes ago => 19.0,
            20 minutes ago => 19.1,
            10 minutes ago => 19.2,
        ];

        let output = calculate_output(mode, current_demand, setpoint, temperatures).unwrap();

        println!("PID output: {:?}", output);

        assert!(output.0 > 40.0, "Expected output to increase heating demand");
        assert_eq!(output.0.fract(), 0.0, "Expected output to be a whole number");
    }
}
