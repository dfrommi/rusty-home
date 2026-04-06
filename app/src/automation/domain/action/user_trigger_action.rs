use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::Command;
use crate::core::domain::HeatingZone;
use crate::core::time::Duration;
use crate::home_state::{FanActivity, PowerAvailable};
use crate::t;
use crate::trigger::*;

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
                tracing::info!("User-trigger action disabled, skipping");
                return Ok(RuleResult::Skip);
            }
        };

        let Some(latest_trigger) = ctx.latest_trigger(self.target.clone()) else {
            tracing::info!("No user-trigger found, skipping");
            return Ok(RuleResult::Skip);
        };

        if latest_trigger.timestamp.elapsed() > trigger_max_duration {
            tracing::info!("Trigger older than {trigger_max_duration}, skipping");
            return Ok(RuleResult::Skip);
        }

        if self.is_one_shot() && latest_trigger.execution_started() {
            tracing::info!("One-shot trigger already executed, skipping");
            return Ok(RuleResult::Skip);
        }

        let command = into_command(&latest_trigger.trigger);

        let Some(command) = command else {
            tracing::info!("Trigger not handled by this action, skipping");
            return Ok(RuleResult::Skip);
        };

        tracing::info!("User-trigger accepted");

        Ok(RuleResult::ExecuteTrigger(command, latest_trigger.id.clone()))
    }
}

impl UserTriggerAction {
    fn default_duration(&self, ctx: &RuleEvaluationContext) -> Option<Duration> {
        match self.target {
            UserTriggerTarget::DevicePower(OnOffDevice::InfraredHeater) => Some(t!(30 minutes)),
            UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier) => Some(t!(15 minutes)),
            UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvEnergySaving) => {
                match ctx.current_dp(PowerAvailable::LivingRoomTv) {
                    Ok(dp) if dp.value => Some(dp.timestamp.elapsed()),
                    Ok(_) => None,
                    Err(e) => {
                        tracing::error!("Error getting current state of living room tv: {:?}", e);
                        None
                    }
                }
            }
            UserTriggerTarget::FanSpeed(FanActivity::LivingRoomCeilingFan)
            | UserTriggerTarget::FanSpeed(FanActivity::BedroomCeilingFan) => Some(t!(10 hours)),
            UserTriggerTarget::FanSpeed(FanActivity::BedroomDehumidifier) => Some(t!(1 hours)),
            UserTriggerTarget::Heating(HeatingZone::LivingRoom)
            | UserTriggerTarget::Heating(HeatingZone::Bedroom)
            | UserTriggerTarget::Heating(HeatingZone::Kitchen)
            | UserTriggerTarget::Heating(HeatingZone::RoomOfRequirements) => None,
            UserTriggerTarget::Heating(HeatingZone::Bathroom) => Some(t!(30 minutes)),
            UserTriggerTarget::Remote(RemoteTriggerTarget::BedroomDoorRemote) => Some(t!(60 minutes)),
            UserTriggerTarget::OpenDoor(_) => Some(t!(30 seconds)),
        }
    }

    fn is_one_shot(&self) -> bool {
        matches!(self.target, UserTriggerTarget::OpenDoor(_))
    }
}

fn into_command(trigger: &UserTrigger) -> Option<Command> {
    use crate::command::*;

    match trigger.clone() {
        UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on,
        } => Some(Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }),
        UserTrigger::DevicePower {
            device: OnOffDevice::Dehumidifier,
            on,
        } => Some(Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }),
        UserTrigger::DevicePower {
            device: OnOffDevice::LivingRoomTvEnergySaving,
            on,
        } => Some(Command::SetEnergySaving {
            device: EnergySavingDevice::LivingRoomTv,
            on,
        }),
        UserTrigger::FanSpeed { fan, airflow } => {
            let device = match fan {
                FanActivity::LivingRoomCeilingFan => Fan::LivingRoomCeilingFan,
                FanActivity::BedroomCeilingFan => Fan::BedroomCeilingFan,
                FanActivity::BedroomDehumidifier => Fan::BedroomDehumidifier,
            };
            Some(Command::ControlFan { device, speed: airflow })
        }
        UserTrigger::Heating { .. } => {
            tracing::info!("Heating state trigger handled elsewhere, skipping");
            None
        }
        UserTrigger::Remote(RemoteTrigger::BedroomDoorRemote(_)) => None,
        UserTrigger::OpenDoor { door: Door::Building } => Some(Command::OpenDoor {
            device: Lock::BuildingEntrance,
        }),
    }
}
