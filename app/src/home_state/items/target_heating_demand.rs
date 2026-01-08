use crate::{
    automation::Thermostat,
    core::{
        timeseries::{DataFrame, DataPoint},
        unit::Percent,
    },
    home_state::{
        AdjustmentDirection, HeatingMode, PidOutput, TargetHeatingAdjustment, TargetHeatingMode,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingDemand {
    Thermostat(Thermostat),
    ByRadiatorTemperature(Thermostat),
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<TargetHeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: TargetHeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        let thermostat = match id {
            TargetHeatingDemand::Thermostat(thermostat) => thermostat,
            TargetHeatingDemand::ByRadiatorTemperature(thermostat) => thermostat,
        };

        let mode = ctx.get(TargetHeatingMode::from_thermostat(thermostat))?;
        let current_demand = ctx.get(thermostat.heating_demand())?;
        let adjustments = ctx.all_since(
            TargetHeatingAdjustment::HeatingDemand(thermostat),
            current_demand.timestamp.max(t!(30 minutes ago)),
        )?;

        match id {
            TargetHeatingDemand::Thermostat(_) => demand_from_pid(&thermostat, mode, ctx),
            TargetHeatingDemand::ByRadiatorTemperature(_) => {
                combined_demand(&thermostat, mode, adjustments, current_demand)
            }
        }
    }
}

struct ControlLimits {
    barely_warm: Percent,
    step: Percent,
    min_output: Percent,
    max_output: Percent,
}

fn combined_demand(
    thermostat: &Thermostat,
    mode: DataPoint<HeatingMode>,
    adjustments: DataFrame<AdjustmentDirection>,
    current_demand: DataPoint<Percent>,
) -> Option<Percent> {
    let adjustment = adjustments.last()?.value.clone();

    let limits = ControlLimits {
        barely_warm: Percent(match thermostat {
            Thermostat::LivingRoomBig => 16.0,
            Thermostat::LivingRoomSmall => 18.0,
            Thermostat::Bedroom => 16.0,
            Thermostat::Kitchen => 18.0,
            Thermostat::RoomOfRequirements => 14.0,
            _ => 16.0,
        }),
        step: Percent(2.0),
        min_output: Percent(8.0),
        max_output: match &mode.value {
            HeatingMode::Ventilation => Percent(0.0),
            HeatingMode::PostVentilation => Percent(20.0),
            HeatingMode::EnergySaving => Percent(40.0),
            HeatingMode::Comfort => Percent(50.0),
            HeatingMode::Manual(_, _) => Percent(60.0),
            HeatingMode::Sleep => Percent(40.0),
            HeatingMode::Away => Percent(30.0),
        },
    };

    if limits.max_output <= Percent(0.0) {
        return Some(Percent(0.0));
    }

    //Don't skip on mode change
    if !adjustment_needed(&adjustments, &current_demand, &mode) {
        return Some(current_demand.value);
    }

    let mut output = current_demand.value;

    match adjustment {
        AdjustmentDirection::MustOff => {
            output = Percent(0.0);
        }
        AdjustmentDirection::MustDecrease | AdjustmentDirection::ShouldDecrease => {
            output = output - limits.step;
        }
        AdjustmentDirection::MustIncrease => {
            output = if output <= Percent(0.0) {
                limits.barely_warm + 2.0 * limits.step
            } else {
                output + limits.step
            };
        }
        AdjustmentDirection::ShouldIncrease => {
            output = if output <= Percent(0.0) {
                limits.barely_warm + limits.step
            } else {
                output + limits.step
            };
        }
        AdjustmentDirection::Hold => {
            //no change
        }
    }

    let output = Percent(output.0.clamp(0.0, limits.max_output.0)).round();
    if output < limits.min_output {
        return Some(Percent(0.0));
    }

    Some(output.clamp())
}

//Gap: current_demand is not necessarily updated when adjustment was applied (keeps same value)
fn adjustment_needed(
    adjustments: &DataFrame<AdjustmentDirection>,
    current_demand: &DataPoint<Percent>,
    mode: &DataPoint<HeatingMode>,
) -> bool {
    //Avoid flood of adjustments
    if current_demand.timestamp.elapsed() < t!(30 seconds) {
        return false;
    }

    //Mode change since last adjustment?
    let new_mode_after_last_change = mode.timestamp > current_demand.timestamp;

    //Adjust every 10 minutes at least
    let min_adjustment_time_reached = current_demand.timestamp.elapsed() > t!(10 minutes);

    //Same direction already applied?
    let mut new_adjustment_after_last_change = false;
    let mut new_adjustment_in_same_direction = false;
    if let Some((first, second)) = adjustments.last2() {
        use AdjustmentDirection::*;

        new_adjustment_after_last_change = second.timestamp > current_demand.timestamp;

        if matches!(first.value, MustIncrease | ShouldIncrease | Hold)
            && matches!(second.value, MustDecrease | ShouldDecrease | Hold)
        {
            new_adjustment_in_same_direction = true;
        }

        if matches!(first.value, MustDecrease | ShouldDecrease | Hold)
            && matches!(second.value, MustIncrease | ShouldIncrease | Hold)
        {
            new_adjustment_in_same_direction = true;
        }

        if first.value == MustOff && second.value == MustOff {
            new_adjustment_in_same_direction = true;
        }
    }

    new_mode_after_last_change
        || min_adjustment_time_reached
        || (new_adjustment_after_last_change && !new_adjustment_in_same_direction)
}

fn demand_from_pid(
    thermostat: &Thermostat,
    mode: DataPoint<HeatingMode>,
    ctx: &StateCalculationContext,
) -> Option<Percent> {
    let force_off_below = match (thermostat, &mode.value) {
        (_, HeatingMode::Ventilation) => Percent(0.0),
        (Thermostat::Kitchen, _) => Percent(17.0),
        (_, _) => Percent(10.0),
    };

    let max_output = match (thermostat, &mode.value) {
        (_, HeatingMode::Ventilation) => Percent(0.0),
        (_, HeatingMode::PostVentilation) => Percent(20.0),
        (Thermostat::Kitchen, HeatingMode::EnergySaving) => Percent(40.0),
        (_, HeatingMode::EnergySaving) => Percent(60.0),
        (_, HeatingMode::Comfort) => Percent(80.0),
        (_, HeatingMode::Manual(_, _)) => Percent(80.0),
        (_, HeatingMode::Sleep) => Percent(50.0),
        (_, HeatingMode::Away) => Percent(60.0),
    };

    let current_demand = ctx.get(thermostat.heating_demand())?;

    let pid_output_id = PidOutput::Thermostat(*thermostat);
    let raw_pid = ctx.get(pid_output_id).map(|pid| pid.value.total())?;
    let heating_demand = raw_pid.round().clamp();

    Some(reduce_valve_movements(
        heating_demand,
        current_demand,
        (force_off_below, max_output),
    ))
}

fn reduce_valve_movements(
    target_demand: Percent,
    current_demand: DataPoint<Percent>,
    allowed_range: (Percent, Percent),
) -> Percent {
    let off = Percent(0.0);
    let significant_change = Percent(10.0);
    let fallback = current_demand.value;

    let keep_duration = t!(10 minutes);

    if target_demand < allowed_range.0 {
        return off;
    }

    let mut output = Percent(target_demand.0.clamp(allowed_range.0.0, allowed_range.1.0));
    let diff = (output - current_demand.value).abs();

    if current_demand.timestamp.elapsed() < keep_duration && diff < significant_change {
        output = fallback;
    }

    output
}
