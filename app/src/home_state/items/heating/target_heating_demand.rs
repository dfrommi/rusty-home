use crate::{
    core::domain::Radiator,
    core::{
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
        unit::Percent,
    },
    home_state::{
        AdjustmentDirection, HeatingDemand, HeatingDemandLimit, HeatingMode, TargetHeatingAdjustment,
        TargetHeatingMode,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};
use r#macro::{EnumVariants, Id};

use super::ControlLimits;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingDemand {
    ControlAndObserve(Radiator),
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<TargetHeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: TargetHeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        let TargetHeatingDemand::ControlAndObserve(radiator) = id;

        let mode = ctx.get(TargetHeatingMode::from_radiator(radiator))?;

        let reference_demand = ctx.get(HeatingDemandLimit::Current(radiator))?;
        let last_change_time = reference_demand.timestamp;
        let adjustments = ctx.all_since(
            TargetHeatingAdjustment::HeatingDemand(radiator),
            last_change_time.max(t!(30 minutes ago)),
        )?;
        let barely_warm_output = ctx.get(HeatingDemand::BarelyWarmSurface(radiator))?.value;

        let is_heating_now = ctx
            .get(radiator.current_heating_demand())
            .map(|d| d.value > Percent(0.0))
            .unwrap_or(false);

        combined_demand(
            radiator,
            mode,
            adjustments,
            is_heating_now,
            barely_warm_output,
            reference_demand,
        )
    }
}

fn combined_demand(
    radiator: Radiator,
    mode: DataPoint<HeatingMode>,
    adjustments: DataFrame<AdjustmentDirection>,
    is_heating_now: bool,

    barely_warm_output: Percent,
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
    }

    if new_adjustment_after_last_change && !new_adjustment_in_same_direction {
        tracing::debug!("New adjustment in different direction since last change. Adjustment needed.");
        return true;
    }

    tracing::debug!("No new adjustment needed since last change.");

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use AdjustmentDirection::*;

    #[tokio::test]
    async fn active_demand_keeps_the_steps_limits_and_immediate_off() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        now.eval_timeshifted(async {
            for (adjustment, current, expected) in [
                (MustIncrease, 20.0, 25.0),
                (ShouldIncrease, 20.0, 25.0),
                (ShouldDecrease, 20.0, 15.0),
                (MustDecrease, 12.0, 12.0),
                (MustIncrease, 50.0, 50.0),
                (MustOff, 50.0, 0.0),
                (Hold, 20.0, 20.0),
            ] {
                let demand = combined_demand(
                    Radiator::LivingRoomBig,
                    DataPoint::new(HeatingMode::Comfort, t!(1 hours ago)),
                    DataFrame::new([DataPoint::new(adjustment, t!(now))]),
                    true,
                    Percent(20.0),
                    DataPoint::new(Percent(current), t!(11 minutes ago)),
                );
                assert_eq!(demand, Some(Percent(expected)));
            }
        })
        .await;
    }

    #[tokio::test]
    async fn idle_radiator_arms_the_barely_warm_on_position() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        now.eval_timeshifted(async {
            for adjustment in [MustDecrease, ShouldDecrease, Hold] {
                let demand = combined_demand(
                    Radiator::LivingRoomBig,
                    DataPoint::new(HeatingMode::Comfort, t!(1 hours ago)),
                    DataFrame::new([DataPoint::new(adjustment, t!(now))]),
                    false,
                    Percent(20.0),
                    DataPoint::new(Percent(35.0), t!(now)),
                );
                assert_eq!(demand, Some(Percent(20.0)));
            }
        })
        .await;
    }

    #[tokio::test]
    async fn retained_increase_adjustments_still_use_cold_start_factors() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        now.eval_timeshifted(async {
            for (adjustment, expected) in [(MustIncrease, 30.0), (ShouldIncrease, 25.0)] {
                let demand = combined_demand(
                    Radiator::LivingRoomBig,
                    DataPoint::new(HeatingMode::Comfort, t!(1 hours ago)),
                    DataFrame::new([DataPoint::new(adjustment, t!(20 minutes ago))]),
                    false,
                    Percent(20.0),
                    DataPoint::new(Percent(35.0), t!(11 minutes ago)),
                );
                assert_eq!(demand, Some(Percent(expected)));
            }
        })
        .await;
    }
}
