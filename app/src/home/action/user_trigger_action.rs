use r#macro::Id;

use crate::adapter::homekit::{HomekitCommand, HomekitCommandTarget, HomekitHeatingState};
use crate::core::time::Duration;
use crate::home::action::{Rule, RuleEvaluationContext, RuleResult};
use crate::home::command::{Command, HeatingTargetState};
use crate::home::common::HeatingZone;
use crate::home::trigger::{ButtonPress, Remote, RemoteTarget, UserTrigger, UserTriggerTarget};
use crate::home_state::PowerAvailable;
use crate::t;

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
            UserTriggerTarget::Remote(RemoteTarget::BedroomDoor)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower) => Some(t!(30 minutes)),
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
            | UserTriggerTarget::Homekit(HomekitCommandTarget::RoomOfRequirementsHeatingState) => Some(t!(1 hours)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::BathroomHeatingState) => Some(t!(30 minutes)),
        }
    }
}

fn into_command(trigger: &UserTrigger) -> Vec<Command> {
    use crate::home::command::*;

    match trigger.clone() {
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::TopSingle)) => vec![Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: true,
        }],
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::BottomSingle)) => vec![Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: false,
        }],
        UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(on)) => vec![Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }],
        UserTrigger::Homekit(HomekitCommand::DehumidifierPower(on)) => vec![Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }],
        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(on)) if !on => vec![Command::SetEnergySaving {
            device: EnergySavingDevice::LivingRoomTv,
            on: false,
        }],
        //Active EnergySaving is just setting back to FollowDefault action
        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(_)) => vec![],
        UserTrigger::Homekit(HomekitCommand::LivingRoomCeilingFanSpeed(speed)) => vec![Command::ControlFan {
            device: Fan::LivingRoomCeilingFan,
            speed,
        }],
        UserTrigger::Homekit(HomekitCommand::BedroomCeilingFanSpeed(speed)) => vec![Command::ControlFan {
            device: Fan::BedroomCeilingFan,
            speed,
        }],
        UserTrigger::Homekit(HomekitCommand::LivingRoomHeatingState(state)) => {
            homekit_heating_actions(HeatingZone::LivingRoom, state)
        }
        UserTrigger::Homekit(HomekitCommand::BedroomHeatingState(state)) => {
            homekit_heating_actions(HeatingZone::Bedroom, state)
        }
        UserTrigger::Homekit(HomekitCommand::KitchenHeatingState(state)) => {
            homekit_heating_actions(HeatingZone::Kitchen, state)
        }
        UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(state)) => {
            homekit_heating_actions(HeatingZone::RoomOfRequirements, state)
        }
        UserTrigger::Homekit(HomekitCommand::BathroomHeatingState(state)) => {
            homekit_heating_actions(HeatingZone::Bathroom, state)
        }
    }
}

fn homekit_heating_actions(zone: HeatingZone, state: HomekitHeatingState) -> Vec<Command> {
    let target_state = match state {
        HomekitHeatingState::Off => HeatingTargetState::Off,
        HomekitHeatingState::Heat(temperature) => HeatingTargetState::Heat {
            temperature,
            low_priority: false,
        },
        HomekitHeatingState::Auto => return vec![],
    };

    zone.thermostats()
        .iter()
        .map(|thermostat| Command::SetHeating {
            device: thermostat.clone(),
            target_state: target_state.clone(),
        })
        .collect()
}
