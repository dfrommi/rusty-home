use crate::{
    automation::Radiator,
    core::{
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint},
        unit::{DegreeCelsius, Percent, RateOfChange},
    },
    home_state::{
        AdjustmentDirection, HeatingDemand, HeatingDemandLimit, HeatingMode, TargetHeatingAdjustment,
        TargetHeatingMode, TemperatureChange,
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

        let Some(modes) = ctx.all_since(TargetHeatingMode::from_radiator(radiator), t!(3 hours ago)) else {
            tracing::warn!(%radiator, "No mode found for radiator {} in the last 3 hours, cannot calculate target heating demand", radiator);
            return None;
        };
        let Some(mode) = modes.last() else {
            tracing::warn!(%radiator, "No mode found for radiator {} in the last 3 hours, cannot calculate target heating demand", radiator);
            return None;
        };

        let reference_demand = ctx
            .get(HeatingDemandLimit::Current(radiator))
            .map(|limit| DataPoint::new(*limit.value.to(), limit.timestamp))?;
        let last_change_time = reference_demand.timestamp;
        let adjustments = ctx.all_since(
            TargetHeatingAdjustment::HeatingDemand(radiator),
            last_change_time.max(t!(30 minutes ago)),
        )?;
        let barely_warm_output = ctx.get(HeatingDemand::BarelyWarmSurface(radiator))?.value;
        let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;
        let coldstart_delay = match (&mode.value, &radiator) {
            (_, Radiator::LivingRoomSmall) => Some(t!(20 minutes)),
            _ => None,
        };

        let is_heating_now = ctx
            .get(radiator.current_heating_demand())
            .map(|d| d.value > Percent(0.0))
            .unwrap_or(false);
        let recent_ventilation_finished = modes.fulfilled_since(|dp| dp.value != HeatingMode::Ventilation);

        combined_demand(
            radiator,
            mode.clone(),
            adjustments,
            is_heating_now,
            recent_ventilation_finished,
            barely_warm_output,
            radiator_roc,
            coldstart_delay,
            reference_demand,
        )
    }
}

struct ControlLimits {
    barely_warm: Percent,
    step: Percent,
    cold_start_should_factor: f64,
    cold_start_must_factor: f64,
    min_output: Percent,
    max_output: Percent,
}

impl ControlLimits {
    fn new(radiator: &Radiator, mode: &HeatingMode, barely_warm_output: Percent) -> Self {
        let (min_output, step) = match radiator { 
            Radiator::LivingRoomBig | Radiator::LivingRoomSmall | Radiator::RoomOfRequirements => (Percent(12.0), Percent(5.0)),
            Radiator::Bedroom | Radiator::Kitchen => (Percent(6.0), Percent(3.0)),
            Radiator::Bathroom => (Percent(10.0), Percent(7.0)),
        };

        Self {
            barely_warm: barely_warm_output,
            step,
            cold_start_should_factor: match mode {
                HeatingMode::Comfort | HeatingMode::Manual(_, _) => 1.0,
                _ => 0.0,
            },
            cold_start_must_factor: match mode {
                HeatingMode::Comfort | HeatingMode::Manual(_, _) => 2.0,
                _ => 0.0,
            },
            min_output,
            max_output: match mode {
                HeatingMode::Ventilation => Percent(0.0),
                HeatingMode::EnergySaving => Percent(40.0),
                HeatingMode::Comfort => Percent(50.0),
                HeatingMode::Manual(_, _) => Percent(60.0),
                HeatingMode::Sleep => Percent(40.0),
                HeatingMode::Away => Percent(30.0),
            }
            .max(min_output),
        }
    }

    fn clamp(&self, output: Percent) -> Percent {
        let output = output.round();
        if output < self.min_output {
            self.min_output
        } else if output > self.max_output {
            self.max_output
        } else {
            output
        }
        .clamp()
    }
}

fn combined_demand(
    radiator: Radiator,
    mode: DataPoint<HeatingMode>,
    adjustments: DataFrame<AdjustmentDirection>,
    is_heating_now: bool,
    recent_ventilation_finished: Option<DateTime>,
    barely_warm_output: Percent,
    radiator_roc: RateOfChange<DegreeCelsius>,
    coldstart_delay: Option<Duration>,
    reference_demand: DataPoint<Percent>,
) -> Option<Percent> {
    let adjustment = adjustments.last()?.value.clone();

    let limits = ControlLimits::new(&radiator, &mode.value, barely_warm_output);

    if adjustment == AdjustmentDirection::MustOff {
        tracing::debug!(%radiator, "Radiator {} must off -> setting output to 0%", radiator);
        return Some(Percent(0.0));
    }

    if !is_heating_now && adjustment <= AdjustmentDirection::Hold {
        tracing::debug!(
            %radiator, 
            "Radiator {} already not heating currently and adjustment is down ({:?}) -> resetting output to barely warm {}", radiator, adjustment, limits.barely_warm);
        return Some(limits.barely_warm);
    }

    if let Some(ventilation_finished) = recent_ventilation_finished {
        let heat_request_after_vent = adjustments.latest_where(|dp| dp.value >= AdjustmentDirection::Hold)
            .take_if(|dp| dp.timestamp >= ventilation_finished)
            .is_some();

        if !is_heating_now && !heat_request_after_vent {
            tracing::debug!(
                %radiator, 
                "Radiator {} recently finished ventilation, no heat request since then -> forcing zero output", radiator);
            return Some(Percent(0.0));
        }
    }

    //Not heating currently, but heat is requested. Wait until delay passed
    //Used for rooms with 2 radiators to not always turn on both at the same time
    //TODO this will not work now with setpoint-based control
    // if let Some(coldstart_delay) = coldstart_delay {
    //     let heat_requested_since = adjustments
    //         .fulfilled_since(|dp| dp.value > AdjustmentDirection::Hold)
    //         .map(|dp| dp.timestamp);
    //     if !is_heating_now && heat_requested_since.is_some_and(|since| since.elapsed() < coldstart_delay) {
    //         return Some(Percent(0.0));
    //     }
    // }

    //Heating present, but temperature on radiator still dropping -> not enough open to release heat
    //Heat up to produce heat again
    //TODO triggers during post_ventilation due to significant temperature drop
    // if heating_but_no_effect(is_heating_now, &radiator_roc) && adjustment <= AdjustmentDirection::Hold {
    //     return Some(limits.clamp(reference_demand.value + limits.step));
    // }

    //Don't skip on mode change
    if !adjustment_needed(&adjustments, &reference_demand.timestamp, &mode) {
        tracing::debug!(
            %radiator, 
            last_changed_at = reference_demand.timestamp.elapsed().to_iso_string(),
            "No adjustment needed for {:?}", radiator);
        return Some(reference_demand.value);
    }

    let mut output = reference_demand.value;
    let is_coldstart = !is_heating_now;

    match adjustment {
        AdjustmentDirection::MustOff => {
            unreachable!("MustOff state handled already")
        }
        AdjustmentDirection::MustDecrease | AdjustmentDirection::ShouldDecrease => {
            output = output - limits.step;
            tracing::debug!(
                %radiator, 
                "Radiator {} decreasing output by {} to {}", radiator, limits.step, output);
        }
        AdjustmentDirection::MustIncrease => {
            if is_coldstart {
                output = limits.barely_warm + limits.cold_start_must_factor * limits.step;
                tracing::debug!(
                    %radiator, 
                    "Radiator {} forced cold-start starting at {} (barely warm {} + factor {} * step {})", 
                    radiator, output, limits.barely_warm, limits.cold_start_must_factor, limits.step);
            } else {
                tracing::debug!(
                    %radiator, 
                    "Radiator {} must increasing output by {} to {}", radiator, limits.step, output + limits.step);
                output = output + limits.step;
            };
        }
        AdjustmentDirection::ShouldIncrease => {
            if is_coldstart {
                output = limits.barely_warm + limits.cold_start_should_factor * limits.step;
                tracing::debug!(
                    %radiator, 
                    "Radiator {} gentle cold-start starting at {} (barely warm {} + factor {} * step {})", 
                    radiator, output, limits.barely_warm, limits.cold_start_should_factor, limits.step);
            } else {
                tracing::debug!(
                    %radiator, 
                    "Radiator {} should increasing output by {} to {}", radiator, limits.step, output + limits.step);
                output = output + limits.step;
            };
        }
        AdjustmentDirection::Hold => {
            tracing::debug!(%radiator, "Radiator {} hold at {}", radiator, output);
            //no change
        }
    }

    Some(limits.clamp(output))
}

//Gap: current_demand is not necessarily updated when adjustment was applied (keeps same value)
//Maybe that belongs to the automation?
fn adjustment_needed(
    adjustments: &DataFrame<AdjustmentDirection>,
    demand_last_changed: &DateTime,
    mode: &DataPoint<HeatingMode>,
) -> bool {
    //Avoid flood of adjustments
    if demand_last_changed.elapsed() < t!(30 seconds) {
        tracing::debug!(
            "Last demand change was {} ago, skipping adjustment",
            demand_last_changed.elapsed()
        );
        return false;
    }

    //Mode change since last adjustment?
    if mode.timestamp > *demand_last_changed {
        tracing::debug!("New heating mode since last change. Adjustment needed.");
        return true;
    }

    //Adjust every 10 minutes at least
    if demand_last_changed.elapsed() > t!(10 minutes) {
        tracing::debug!("Last demand change was more than 10 minutes ago. Adjustment needed.");
        return true;
    }

    //Same direction already applied?
    let mut new_adjustment_after_last_change = false;
    let mut new_adjustment_in_same_direction = false;
    if let Some((previous, latest)) = adjustments.last2() {
        use AdjustmentDirection::*;

        new_adjustment_after_last_change = latest.timestamp > *demand_last_changed;

        if matches!(previous.value, MustIncrease | ShouldIncrease | Hold)
            && matches!(latest.value, MustIncrease | ShouldIncrease | Hold)
        {
            new_adjustment_in_same_direction = true;
        }

        if matches!(previous.value, MustDecrease | ShouldDecrease | Hold)
            && matches!(latest.value, MustDecrease | ShouldDecrease | Hold)
        {
            new_adjustment_in_same_direction = true;
        }

        if previous.value == MustOff && latest.value == MustOff {
            new_adjustment_in_same_direction = true;
        }
    }

    if new_adjustment_after_last_change && !new_adjustment_in_same_direction {
        tracing::debug!("New adjustment in different direction since last change. Adjustment needed.");
        return true;
    }

    tracing::debug!("No new adjustment needed since last change.");

    false
}


//TODO min elapsed
//TODO clamp to last mode change to avoid issue in post-ventilation
fn heating_but_no_effect(is_heating_now: bool, radiator_roc: &RateOfChange<DegreeCelsius>) -> bool {
    is_heating_now && radiator_roc.per_hour() < DegreeCelsius(-2.0)
}
