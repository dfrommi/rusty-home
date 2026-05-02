use r#macro::{EnumVariants, Id};

use crate::automation::RuleEvaluationContext;
use crate::core::unit::FanAirflow;
use crate::{
    automation::domain::action::{Rule, RuleResult},
    command::{Command, Fan, PowerToggle},
    home_state::FanActivity,
    home_state::Resident,
    t,
    trigger::{OnOffDevice, RemoteTriggerTarget, UserTriggerTarget},
};

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum BlockAutomation {
    BathroomDehumidifier,
    BedroomDehumidifier,
}

impl Rule for BlockAutomation {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let sleeping_start = {
            let sleeping = ctx.current_dp(Resident::AnyoneSleeping)?;
            if sleeping.value { Some(sleeping.timestamp) } else { None }
        };

        let blocked_start = match self {
            BlockAutomation::BathroomDehumidifier | BlockAutomation::BedroomDehumidifier => {
                let night_time_start = t!(22:00 - 9:00).active().map(|r| *r.start());
                if sleeping_start.is_some() || night_time_start.is_some() {
                    tracing::info!("Sleep mode or night time active; blocking dehumidifier");
                } else {
                    tracing::info!("Not sleeping and not night time; not blocking dehumidifier");
                }
                sleeping_start
                    .map(|s| s.min(night_time_start.unwrap_or(s)))
                    .or(night_time_start)
            }
        };

        let Some(blocked_start) = blocked_start else {
            tracing::info!("No block active; skipping");
            return Ok(RuleResult::Skip);
        };

        // Check if user override exists after block started — if so, skip and let
        // lower-priority rules (e.g. UserTriggerAction) handle the resource.
        let trigger_target = match self {
            BlockAutomation::BathroomDehumidifier => UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier),
            BlockAutomation::BedroomDehumidifier => UserTriggerTarget::FanSpeed(FanActivity::BedroomDehumidifier),
        };

        let trigger = ctx.latest_trigger(trigger_target);
        if let Some(trigger) = trigger
            && trigger.timestamp > blocked_start
        {
            tracing::info!("User override detected after block started; yielding to lower-priority rules");
            return Ok(RuleResult::Skip);
        }

        // Also check remote trigger for bedroom devices
        if matches!(self, BlockAutomation::BedroomDehumidifier) {
            let remote_trigger = ctx.latest_trigger(UserTriggerTarget::Remote(RemoteTriggerTarget::BedroomDoorRemote));
            if let Some(trigger) = remote_trigger
                && trigger.timestamp > blocked_start
            {
                tracing::info!("Remote override detected after block started; yielding to lower-priority rules");
                return Ok(RuleResult::Skip);
            }
        }

        // No override — produce the off command directly.
        tracing::info!("No valid user override; turning off device");
        let command = match self {
            BlockAutomation::BathroomDehumidifier => Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            },
            BlockAutomation::BedroomDehumidifier => Command::ControlFan {
                device: Fan::BedroomDehumidifier,
                speed: FanAirflow::Off,
            },
        };

        Ok(RuleResult::Execute(command))
    }
}
