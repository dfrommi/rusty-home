use crate::{
    automation::Thermostat,
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Interpolator, LastSeenInterpolator, LinearInterpolator},
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
    fn calculate_current(&self, id: TargetHeatingDemand, ctx: &StateCalculationContext) -> Option<DataPoint<Percent>> {
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

        let mode = ctx.get(mode)?;

        let pid_params = get_pid_params_for_mode(thermostat, &mode.value);
        let output = match mode.value {
            HeatingMode::Ventilation => Some(mode.with(Percent(0.0))),

            _ => {
                let start_at = mode.timestamp;
                let setpoints = match ctx.all_since(thermostat.set_point(), start_at) {
                    Some(value) => value,
                    None => return fallback_heating_demand(ctx, thermostat),
                };
                let temperatures = match ctx.all_since(temperature_id, start_at) {
                    Some(value) => value,
                    None => return fallback_heating_demand(ctx, thermostat),
                };
                let output = calculate_pid(&pid_params, &setpoints, &temperatures, mode.timestamp, t!(30 seconds))
                    .map(|value| DataPoint::new(value, t!(now)));
                output.or_else(|| fallback_heating_demand(ctx, thermostat))
            }
        };

        output.map(|value| normalize_output(&pid_params, value))
    }
}

struct PidParams {
    kp: f64,
    ki: f64,
    kd: f64,
    min_output: Percent,
    max_output: Percent,
    deadband: DegreeCelsius,
}

fn get_pid_params_for_mode(_thermostat: Thermostat, mode: &HeatingMode) -> PidParams {
    match mode {
        HeatingMode::EnergySaving => PidParams {
            kp: 35.0,
            ki: 8.0,
            kd: 4.0,
            min_output: Percent(10.0),
            max_output: Percent(80.0),
            deadband: DegreeCelsius(0.3),
        },
        HeatingMode::Comfort => PidParams {
            kp: 50.0,
            ki: 10.0,
            kd: 5.0,
            min_output: Percent(10.0),
            max_output: Percent(100.0),
            deadband: DegreeCelsius(0.2),
        },
        HeatingMode::Sleep => PidParams {
            kp: 20.0,
            ki: 4.0,
            kd: 2.0,
            min_output: Percent(10.0),
            max_output: Percent(60.0),
            deadband: DegreeCelsius(0.4),
        },
        HeatingMode::Away => PidParams {
            kp: 15.0,
            ki: 3.0,
            kd: 2.0,
            min_output: Percent(10.0),
            max_output: Percent(40.0),
            deadband: DegreeCelsius(0.5),
        },
        HeatingMode::Ventilation => PidParams {
            kp: 0.0,
            ki: 0.0,
            kd: 0.0,
            min_output: Percent(0.0),
            max_output: Percent(0.0),
            deadband: DegreeCelsius(0.0),
        },
        HeatingMode::PostVentilation => PidParams {
            kp: 30.0,
            ki: 6.0,
            kd: 3.0,
            min_output: Percent(10.0),
            max_output: Percent(25.0),
            deadband: DegreeCelsius(0.25),
        },
        HeatingMode::Manual(_, _) => PidParams {
            kp: 70.0,
            ki: 15.0,
            kd: 6.0,
            min_output: Percent(10.0),
            max_output: Percent(100.0),
            deadband: DegreeCelsius(0.1),
        },
    }
}

fn calculate_pid(
    params: &PidParams,
    setpoints: &DataFrame<DegreeCelsius>,
    temperatures: &DataFrame<DegreeCelsius>,
    start_at: DateTime,
    step: Duration,
) -> Option<Percent> {
    let history_start = (t!(now) - t!(3 hours)).max(start_at);
    let range = DateTimeRange::since(history_start);
    let step_hours = step.as_hours_f64().max(1e-6);
    let mut integral = 0.0;
    let mut prev_error: Option<f64> = None;
    let mut output = 0.0;
    let output_cap = params.max_output.0.min(100.0);
    let integral_cap = if params.ki.abs() > 1e-9 {
        output_cap / params.ki.abs()
    } else {
        0.0
    };
    let mut sample_count = 0;

    for dt in range.step_by(step) {
        let setpoint = match LastSeenInterpolator.interpolate_df(dt, setpoints).ok().flatten() {
            Some(value) => value,
            None => {
                prev_error = None;
                continue;
            }
        };
        let temperature = match LinearInterpolator.interpolate_df(dt, temperatures).ok().flatten() {
            Some(value) => value,
            None => {
                prev_error = None;
                continue;
            }
        };

        let error = (setpoint - temperature).0;
        if error.abs() <= params.deadband.0 {
            integral = 0.0;
            prev_error = Some(0.0);
            output = 0.0;
            sample_count += 1;
            continue;
        }

        integral = (integral + error * step_hours).clamp(-integral_cap, integral_cap);
        let derivative = prev_error.map(|prev| (error - prev) / step_hours).unwrap_or(0.0);

        output = params.kp * error + params.ki * integral + params.kd * derivative;
        prev_error = Some(error);
        sample_count += 1;
    }

    if sample_count < 2 {
        return None;
    }

    Some(Percent(output))
}

fn fallback_heating_demand(ctx: &StateCalculationContext, thermostat: Thermostat) -> Option<DataPoint<Percent>> {
    ctx.get(thermostat.heating_demand())
}

fn normalize_output(params: &PidParams, output: DataPoint<Percent>) -> DataPoint<Percent> {
    output.map_value(|value| {
        let output_cap = params.max_output.0.min(100.0).max(0.0);
        let clamped = value.0.clamp(0.0, output_cap);
        let quantized = quantize_percent(clamped);
        let min_output = params.min_output.0.max(0.0);
        let final_output = if quantized > 0.0 && quantized < min_output {
            0.0
        } else {
            quantized
        };
        Percent(final_output)
    })
}

fn quantize_percent(value: f64) -> f64 {
    let step = 5.0;
    (value / step).round() * step
}
