use anyhow::{Result, anyhow};
use r#macro::Id;

use crate::{
    command::{Command, HeatingTargetState, SetHeating},
    core::{domain::Radiator, time::DateTime, timeseries::DataPoint, unit::Percent},
    home_state::{AdjustmentDirection, HeatingMode, TargetHeatingAdjustment, TargetHeatingMode, Ventilation},
    t,
    trigger::UserTriggerTarget,
};

use super::{RuleEvaluationContext, SimpleRule};

#[derive(Debug, Clone, Id)]
pub struct PauseHeatingForVentilation {
    radiator: Radiator,
}

impl PauseHeatingForVentilation {
    pub fn new(radiator: Radiator) -> Self {
        Self { radiator }
    }
}

impl SimpleRule for PauseHeatingForVentilation {
    fn command(&self) -> Command {
        SetHeating {
            device: self.radiator,
            target_state: HeatingTargetState::Off,
        }
        .into()
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool> {
        let zone = self.radiator.heating_zone();
        let ventilation = ctx.current_dp(Ventilation::Room(zone.room_with_window()))?;
        let target_mode = ctx.current(TargetHeatingMode::HeatingZone(zone))?;

        let manual_request_ts = match &target_mode {
            HeatingMode::Manual(_, trigger_id) => ctx
                .latest_trigger(UserTriggerTarget::Heating(zone))
                .filter(|trigger| trigger.id == *trigger_id)
                .map(|trigger| trigger.timestamp),
            _ => None,
        };

        // Control inputs may be missing while ventilating; they must not prevent the pause.
        let adjustment = ctx
            .current_dp(TargetHeatingAdjustment::HeatingDemand(self.radiator))
            .ok();
        let is_heating = ctx
            .current(self.radiator.current_heating_demand())
            .is_ok_and(|demand| demand > Percent(0.0));

        should_pause(&ventilation, target_mode, manual_request_ts, adjustment, is_heating)
    }
}

fn should_pause(
    ventilation: &DataPoint<bool>,
    target_mode: HeatingMode,
    manual_request_ts: Option<DateTime>,
    adjustment: Option<DataPoint<AdjustmentDirection>>,
    is_heating: bool,
) -> Result<bool> {
    if manual_request_ts.is_some_and(|timestamp| !ventilation.value || timestamp > ventilation.timestamp) {
        tracing::info!("Manual request started after ventilation; allowing normal heating control");
        return Ok(false);
    }

    if ventilation.value {
        tracing::info!("Ventilation is active; pausing heating");
        return Ok(true);
    }

    let (delay, delay_minutes) = match target_mode {
        HeatingMode::Comfort => (t!(5 minutes), 5),
        _ => (t!(20 minutes), 20),
    };

    if ventilation.timestamp.elapsed() < delay {
        tracing::info!("Ventilation ended less than {delay_minutes} minutes ago; keeping heating paused");
        return Ok(true);
    }

    let adjustment = adjustment.ok_or_else(|| anyhow!("Heating adjustment not found after ventilation"))?;
    if !is_heating && adjustment.value > AdjustmentDirection::Hold && adjustment.timestamp < ventilation.timestamp {
        tracing::info!("Idle radiator still has a pre-ventilation increase request; keeping heating paused");
        return Ok(true);
    }

    tracing::info!("Post-ventilation pause finished; allowing normal heating control");
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::unit::DegreeCelsius, trigger::UserTriggerId};
    use AdjustmentDirection::*;

    fn completed_ventilation(finished: DateTime) -> DataPoint<bool> {
        DataPoint::new(false, finished)
    }

    fn manual_mode() -> HeatingMode {
        HeatingMode::Manual(DegreeCelsius(20.0), UserTriggerId::from(1))
    }

    #[tokio::test]
    async fn only_a_manual_request_strictly_after_activation_bypasses_active_ventilation() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let started = t!(10 minutes ago);
                let ventilation = DataPoint::new(true, started);
                for (manual_request, expected_pause) in [
                    (None, true),
                    (Some(started - t!(1 minutes)), true),
                    (Some(started - t!(10 seconds)), true),
                    (Some(started), true),
                    (Some(started + t!(1 seconds)), false),
                ] {
                    assert_eq!(
                        should_pause(&ventilation, manual_mode(), manual_request, None, false).ok(),
                        Some(expected_pause)
                    );
                }
            })
            .await;
    }

    #[tokio::test]
    async fn an_active_manual_request_is_never_cancelled_after_closing() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let finished = t!(1 minutes ago);
                let ventilation = completed_ventilation(finished);
                for requested in [t!(30 minutes ago), finished - t!(5 minutes), finished, t!(now)] {
                    assert_eq!(
                        should_pause(&ventilation, manual_mode(), Some(requested), None, false).ok(),
                        Some(false)
                    );
                }
            })
            .await;
    }

    #[tokio::test]
    async fn a_new_ventilation_session_invalidates_an_earlier_manual_bypass() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let ventilation = DataPoint::new(true, t!(5 minutes ago));
                assert_eq!(
                    should_pause(&ventilation, manual_mode(), Some(t!(25 minutes ago)), None, false).ok(),
                    Some(true)
                );
                assert_eq!(
                    should_pause(&ventilation, manual_mode(), Some(t!(1 minutes ago)), None, false).ok(),
                    Some(false)
                );
            })
            .await;
    }

    #[tokio::test]
    async fn post_ventilation_pause_uses_the_delay_for_the_target_mode() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                for (mode, before_delay, delay) in [
                    (HeatingMode::Comfort, t!(299 seconds), t!(5 minutes)),
                    (HeatingMode::EnergySaving, t!(1199 seconds), t!(20 minutes)),
                ] {
                    for (elapsed, expected_pause) in [
                        (before_delay, true),
                        (delay.clone(), false),
                        (delay + t!(1 minutes), false),
                    ] {
                        let finished = t!(now) - elapsed.clone();
                        let adjustment = Some(DataPoint::new(Hold, finished));
                        assert_eq!(
                            should_pause(&completed_ventilation(finished), mode.clone(), None, adjustment, false).ok(),
                            Some(expected_pause),
                            "mode {mode}, elapsed {elapsed:?}"
                        );
                    }
                }
            })
            .await;
    }

    #[tokio::test]
    async fn retained_pre_ventilation_increases_cannot_restart_an_idle_radiator() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let finished = t!(10 minutes ago);
                let ventilation = completed_ventilation(finished);
                for direction in [MustIncrease, ShouldIncrease] {
                    let old_request = Some(DataPoint::new(direction, t!(30 minutes ago)));
                    assert_eq!(
                        should_pause(&ventilation, HeatingMode::Comfort, None, old_request.clone(), false).ok(),
                        Some(true)
                    );
                    assert_eq!(
                        should_pause(&ventilation, HeatingMode::Comfort, None, old_request.clone(), true).ok(),
                        Some(false)
                    );
                    assert_eq!(
                        should_pause(&ventilation, HeatingMode::Comfort, Some(t!(15 minutes ago)), old_request, false)
                            .ok(),
                        Some(false)
                    );
                }
                for direction in [MustOff, MustDecrease, ShouldDecrease, Hold] {
                    let adjustment = Some(DataPoint::new(direction, t!(30 minutes ago)));
                    assert_eq!(
                        should_pause(&ventilation, HeatingMode::Comfort, None, adjustment, false).ok(),
                        Some(false)
                    );
                }
            })
            .await;
    }

    #[tokio::test]
    async fn increases_from_the_closing_time_or_later_release_the_pause() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let finished = t!(10 minutes ago);
                let ventilation = completed_ventilation(finished);
                for requested in [finished, t!(1 minutes ago)] {
                    let adjustment = Some(DataPoint::new(MustIncrease, requested));
                    assert_eq!(
                        should_pause(&ventilation, HeatingMode::Comfort, None, adjustment, false).ok(),
                        Some(false)
                    );
                }
            })
            .await;
    }

    #[tokio::test]
    async fn a_first_closed_datapoint_is_treated_like_a_recent_closing() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                let ventilation = DataPoint::new(false, t!(now));
                assert_eq!(
                    should_pause(&ventilation, HeatingMode::Comfort, None, None, false).ok(),
                    Some(true)
                );
            })
            .await;
    }

    #[tokio::test]
    async fn control_data_is_only_required_after_the_mode_specific_pause() {
        DateTime::from_static_iso("2026-01-01T12:00:00Z")
            .eval_timeshifted(async {
                assert_eq!(
                    should_pause(
                        &completed_ventilation(t!(1 minutes ago)),
                        HeatingMode::Comfort,
                        None,
                        None,
                        false
                    )
                    .ok(),
                    Some(true)
                );
                assert!(
                    should_pause(
                        &completed_ventilation(t!(10 minutes ago)),
                        HeatingMode::Comfort,
                        None,
                        None,
                        false
                    )
                    .is_err()
                );
                assert_eq!(
                    should_pause(
                        &completed_ventilation(t!(10 minutes ago)),
                        HeatingMode::EnergySaving,
                        None,
                        None,
                        false
                    )
                    .ok(),
                    Some(true)
                );
            })
            .await;
    }
}
