use std::cmp::max;

use crate::{
    automation::Thermostat,
    core::{timeseries::DataPoint, unit::Percent},
    home_state::{
        HeatingMode, PidOutput, TargetHeatingMode,
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
        let TargetHeatingDemand::Thermostat(thermostat) = id;
        let mode = ctx.get(TargetHeatingMode::from_thermostat(thermostat))?;

        let allowed_range = match mode.value {
            HeatingMode::Ventilation => (Percent(0.0), Percent(0.0)),
            HeatingMode::PostVentilation => (Percent(10.0), Percent(20.0)),
            //Only avoid cooling down too much long term
            HeatingMode::EnergySaving if thermostat == Thermostat::Kitchen => (Percent(13.0), Percent(40.0)),
            HeatingMode::EnergySaving => (Percent(10.0), Percent(60.0)),
            HeatingMode::Comfort => (Percent(10.0), Percent(80.0)),
            HeatingMode::Manual(_, _) => (Percent(10.0), Percent(80.0)),
            HeatingMode::Sleep => (Percent(10.0), Percent(50.0)),
            HeatingMode::Away => (Percent(10.0), Percent(60.0)),
        };

        let current_demand = ctx.get(thermostat.heating_demand())?;

        let pid_output_id = match id {
            TargetHeatingDemand::Thermostat(thermostat) => PidOutput::Thermostat(thermostat),
        };

        let raw_pid = ctx.get(pid_output_id).map(|pid| pid.value.total())?;
        let heating_demand = raw_pid.round().clamp();

        Some(reduce_valve_movements(heating_demand, current_demand, allowed_range))
    }
}

fn reduce_valve_movements(
    target_demand: Percent,
    current_demand: DataPoint<Percent>,
    allowed_range: (Percent, Percent),
) -> Percent {
    let off = Percent(0.0);
    let significant_change = Percent(20.0);
    let fallback = current_demand.value;

    let skip_change_band = Percent(5.0);
    let keep_duration = t!(5 minutes);

    let effectively_off = Percent(3.0);
    //round halfway to effectively off to avoid rapid on/off cycling
    let off_threshold = 0.5 * (allowed_range.0 - effectively_off);

    if target_demand < effectively_off || target_demand < off_threshold {
        return off;
    }

    let mut output = Percent(target_demand.0.clamp(allowed_range.0.0, allowed_range.1.0));
    let diff = (output - current_demand.value).abs();

    if diff < skip_change_band {
        output = current_demand.value;
    }

    if current_demand.timestamp.elapsed() < keep_duration && diff < significant_change {
        output = fallback;
    }

    output
}
