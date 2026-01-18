use crate::{
    automation::Radiator,
    core::{
        timeseries::{DataFrame, DataPoint},
        unit::{DegreeCelsius, Percent, RateOfChange},
    },
    home_state::{
        AdjustmentDirection, HeatingDemand, HeatingMode, TargetHeatingAdjustment, TargetHeatingMode, TemperatureChange,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingDemand {
    ControlAndObserve(Radiator),
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<TargetHeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: TargetHeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        let TargetHeatingDemand::ControlAndObserve(radiator) = id;

        let mode = ctx.get(TargetHeatingMode::from_radiator(radiator))?;
        let current_demand = ctx.get(radiator.heating_demand())?;
        let adjustments = ctx.all_since(
            TargetHeatingAdjustment::HeatingDemand(radiator),
            current_demand.timestamp.max(t!(30 minutes ago)),
        )?;
        let barely_warm_output = ctx.get(HeatingDemand::BarelyWarmSurface(radiator))?.value;
        let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;

        combined_demand(mode, adjustments, current_demand, barely_warm_output, radiator_roc)
    }
}

struct ControlLimits {
    barely_warm: Percent,
    step: Percent,
    min_output: Percent,
    max_output: Percent,
}

fn combined_demand(
    mode: DataPoint<HeatingMode>,
    adjustments: DataFrame<AdjustmentDirection>,
    current_demand: DataPoint<Percent>,
    barely_warm_output: Percent,
    radiator_roc: RateOfChange<DegreeCelsius>,
) -> Option<Percent> {
    let adjustment = adjustments.last()?.value.clone();

    let limits = ControlLimits {
        barely_warm: barely_warm_output,
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

    //Heating present, but temperature on radiator still dropping -> not enough open to release heat
    //Turn off if cooldown intended, but not if anyway in heatup phase already to not interrupt it
    if heating_but_no_effect(&current_demand, &radiator_roc) && adjustment <= AdjustmentDirection::Hold {
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
//Maybe that belongs to the automation?
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
            && matches!(second.value, MustIncrease | ShouldIncrease | Hold)
        {
            new_adjustment_in_same_direction = true;
        }

        if matches!(first.value, MustDecrease | ShouldDecrease | Hold)
            && matches!(second.value, MustDecrease | ShouldDecrease | Hold)
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

fn heating_but_no_effect(current_demand: &DataPoint<Percent>, radiator_roc: &RateOfChange<DegreeCelsius>) -> bool {
    current_demand.value > Percent(0.0) && radiator_roc.per_hour() < DegreeCelsius(-2.0)
}
