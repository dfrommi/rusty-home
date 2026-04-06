use r#macro::{EnumVariants, Id};

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{Command, Fan, PowerToggle};
use crate::core::unit::FanAirflow;
use crate::t;
use crate::trigger::{DualButtonPress, RemoteTrigger, RemoteTriggerTarget, UserTrigger, UserTriggerTarget};

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum RemoteTurnOff {
    InfraredHeater,
    BedroomDehumidifier,
    BedroomCeilingFan,
}

impl Rule for RemoteTurnOff {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let trigger_target = UserTriggerTarget::Remote(RemoteTriggerTarget::BedroomDoorRemote);
        let Some(trigger) = ctx.latest_trigger(trigger_target) else {
            tracing::info!("No remote trigger found, skipping");
            return Ok(RuleResult::Skip);
        };

        if trigger.timestamp.elapsed() > t!(60 minutes) {
            tracing::info!("Remote trigger older than 60 minutes, skipping");
            return Ok(RuleResult::Skip);
        }

        if trigger.execution_started() {
            tracing::info!("Remote trigger already executed, skipping");
            return Ok(RuleResult::Skip);
        }

        let UserTrigger::Remote(RemoteTrigger::BedroomDoorRemote(DualButtonPress::SingleOff)) = &trigger.trigger else {
            tracing::info!("Remote trigger is not SingleOff, skipping");
            return Ok(RuleResult::Skip);
        };

        let command = match self {
            RemoteTurnOff::InfraredHeater => Command::SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            },
            RemoteTurnOff::BedroomDehumidifier => Command::ControlFan {
                device: Fan::BedroomDehumidifier,
                speed: FanAirflow::Off,
            },
            RemoteTurnOff::BedroomCeilingFan => Command::ControlFan {
                device: Fan::BedroomCeilingFan,
                speed: FanAirflow::Off,
            },
        };

        tracing::info!("Remote turn-off accepted");
        Ok(RuleResult::ExecuteTrigger(command, trigger.id.clone()))
    }
}
