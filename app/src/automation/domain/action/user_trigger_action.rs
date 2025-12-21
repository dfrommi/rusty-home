use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::Command;
use crate::core::time::Duration;
use crate::frontends::homekit::{HomekitCommand, HomekitCommandTarget};
use crate::home_state::PowerAvailable;
use crate::t;
use crate::trigger::{UserTrigger, UserTriggerTarget};

#[derive(Debug, Clone, Id)]
pub struct UserTriggerAction {
    target: UserTriggerTarget,
}

impl UserTriggerAction {
    pub fn new(target: UserTriggerTarget) -> Self {
        Self { target }
    }
}

impl Rule for UserTriggerAction {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let trigger_max_duration = match self.default_duration(ctx) {
            Some(duration) => duration,
            None => {
                tracing::trace!("User-trigger action currently disabled, skipping");
                return Ok(RuleResult::Skip);
            }
        };

        let Some(latest_trigger) = ctx.latest_trigger(self.target.clone()) else {
            tracing::trace!("No user-trigger found, skipping");
            return Ok(RuleResult::Skip);
        };

        if latest_trigger.timestamp.elapsed() > trigger_max_duration {
            tracing::trace!("Trigger expired after {}, skipping", trigger_max_duration);
            return Ok(RuleResult::Skip);
        }

        let commands = into_command(&latest_trigger.trigger);

        if commands.is_empty() {
            tracing::trace!("Trigger not handled by this action, skipping");
            return Ok(RuleResult::Skip);
        }

        tracing::trace!(?commands, ?latest_trigger, "User-trigger action(s) ready to be executed");

        Ok(RuleResult::ExecuteTrigger(commands, latest_trigger.id.clone()))
    }
}

impl UserTriggerAction {
    fn default_duration(&self, ctx: &RuleEvaluationContext) -> Option<Duration> {
        match self.target {
            UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower) => Some(t!(30 minutes)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower) => Some(t!(15 minutes)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomTvEnergySaving) => {
                match ctx.current_dp(PowerAvailable::LivingRoomTv) {
                    Ok(dp) if dp.value => Some(dp.timestamp.elapsed()),
                    Ok(_) => None,
                    Err(e) => {
                        tracing::error!("Error getting current state of living room tv: {:?}", e);
                        None
                    }
                }
            }
            UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomCeilingFanSpeed)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomCeilingFanSpeed) => Some(t!(10 hours)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomHeatingState)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomHeatingState)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::KitchenHeatingState)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::RoomOfRequirementsHeatingState) => None,
            UserTriggerTarget::Homekit(HomekitCommandTarget::BathroomHeatingState) => Some(t!(30 minutes)),
        }
    }
}

fn into_command(trigger: &UserTrigger) -> Vec<Command> {
    use crate::command::*;

    match trigger.clone() {
        UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(on)) => vec![Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }],
        UserTrigger::Homekit(HomekitCommand::DehumidifierPower(on)) => vec![Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }],
        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(on)) => vec![Command::SetEnergySaving {
            device: EnergySavingDevice::LivingRoomTv,
            on,
        }],
        UserTrigger::Homekit(HomekitCommand::LivingRoomCeilingFanSpeed(speed)) => vec![Command::ControlFan {
            device: Fan::LivingRoomCeilingFan,
            speed,
        }],
        UserTrigger::Homekit(HomekitCommand::BedroomCeilingFanSpeed(speed)) => vec![Command::ControlFan {
            device: Fan::BedroomCeilingFan,
            speed,
        }],
        UserTrigger::Homekit(HomekitCommand::LivingRoomHeatingState(_))
        | UserTrigger::Homekit(HomekitCommand::BedroomHeatingState(_))
        | UserTrigger::Homekit(HomekitCommand::KitchenHeatingState(_))
        | UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(_))
        | UserTrigger::Homekit(HomekitCommand::BathroomHeatingState(_)) => {
            tracing::warn!("Homekit heating state triggers are handled by FollowTargetHeatingDemand rule, skipping");
            vec![]
        }
    }
}
