use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{PushNotification, PushNotify, PushRecipient, WashingMachineState};
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::home_state::{CompletionTime, IsRunning, PowerAvailable};
use crate::t;

#[derive(Debug, Clone, Id)]
pub enum InformDeviceState {
    WashingMachine(PushRecipient),
}

impl Rule for InformDeviceState {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let InformDeviceState::WashingMachine(recipient) = self;
        let turned_on = ctx.current_dp(PowerAvailable::WashingMachine)?;
        let running = ctx.current_dp(IsRunning::WashingMachine)?;
        let estimated_end = ctx.current(CompletionTime::WashingMachineEstimated)?;
        let state = match washing_machine_state(turned_on, running, estimated_end, t!(now)) {
            Some(state) => state,
            None => return Ok(RuleResult::Skip),
        };

        Ok(RuleResult::Execute(
            PushNotify::Show(*recipient, PushNotification::WashingMachine(state)).into(),
        ))
    }
}

fn washing_machine_state(
    powered_on: DataPoint<bool>,
    running: DataPoint<bool>,
    estimated_end: DateTime,
    now: DateTime,
) -> Option<WashingMachineState> {
    if !powered_on.value {
        tracing::info!("Washing machine is not powered on; skipping notification");
        return None;
    }

    //Still turned on after finished. Capture turn-on before starting program
    if !running.value {
        //program was excuted in between, skipping powered-on while loading machine
        if running.timestamp > powered_on.timestamp {
            tracing::info!("Washing machine no longer running, but still powered on; emitting done-notification");
            return Some(WashingMachineState::Done);
        }

        tracing::info!("Washing machine is powered on, but not yet running; skipping notification");
        return None;
    }

    //running from here on

    let estimated_end = if estimated_end > now {
        tracing::info!("Washing machine is running with a future estimated end");
        estimated_end
    } else {
        tracing::info!("Washing machine estimate is not in the future; using a 2-minute fallback");
        now + t!(2 minutes)
    };

    Some(WashingMachineState::InProgress {
        started_at: running.timestamp,
        estimated_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_a_powered_off_machine() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");

        assert_eq!(
            washing_machine_state(
                DataPoint::new(false, now - t!(30 seconds)),
                DataPoint::new(false, now),
                now + t!(20 minutes),
                now,
            ),
            None
        );
    }

    #[test]
    fn skips_a_machine_that_has_not_started_running() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");
        let powered_on_at = now - t!(30 seconds);

        assert_eq!(
            washing_machine_state(
                DataPoint::new(true, powered_on_at),
                DataPoint::new(false, powered_on_at),
                now + t!(20 minutes),
                now,
            ),
            None
        );
    }

    #[test]
    fn reports_done_after_the_program_stops() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");

        assert_eq!(
            washing_machine_state(
                DataPoint::new(true, now - t!(20 minutes)),
                DataPoint::new(false, now - t!(30 seconds)),
                now,
                now,
            ),
            Some(WashingMachineState::Done)
        );
    }

    #[test]
    fn uses_a_future_estimated_end_for_a_running_machine() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");
        let started_at = now - t!(10 minutes);
        let estimated_end = now + t!(20 minutes);

        assert_eq!(
            washing_machine_state(
                DataPoint::new(true, now - t!(20 minutes)),
                DataPoint::new(true, started_at),
                estimated_end,
                now,
            ),
            Some(WashingMachineState::InProgress {
                started_at,
                estimated_end,
            })
        );
    }

    #[test]
    fn uses_a_two_minute_fallback_for_a_past_estimated_end() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");
        let started_at = now - t!(10 minutes);

        assert_eq!(
            washing_machine_state(
                DataPoint::new(true, now - t!(20 minutes)),
                DataPoint::new(true, started_at),
                now - t!(1 minutes),
                now,
            ),
            Some(WashingMachineState::InProgress {
                started_at,
                estimated_end: now + t!(2 minutes),
            })
        );
    }
}
