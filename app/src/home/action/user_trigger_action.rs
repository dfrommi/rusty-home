use crate::adapter::homekit::{HomekitCommand, HomekitCommandTarget, HomekitHeatingState};
use crate::core::HomeApi;
use crate::core::planner::{Action, ActionEvaluationResult};
use crate::core::time::Duration;
use crate::home::command::{Command, CommandSource, HeatingTargetState};
use crate::home::common::HeatingZone;
use crate::home::state::Powered;
use crate::home::trigger::{ButtonPress, Remote, RemoteTarget, UserTrigger, UserTriggerTarget};
use crate::t;

use super::{DataPointAccess, needs_execution_for_one_shot_of_target};

#[derive(Debug, Clone, derive_more::Display)]
#[display("UserTriggerAction[{}]", target)]
pub struct UserTriggerAction {
    target: UserTriggerTarget,
}

impl UserTriggerAction {
    pub fn new(target: UserTriggerTarget) -> Self {
        Self { target }
    }
}

impl Action for UserTriggerAction {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<ActionEvaluationResult> {
        let start_of_range = match self.default_duration(api).await {
            Some(duration) => t!(now) - duration,
            None => {
                tracing::trace!("User-trigger action currently disabled, skipping");
                return Ok(ActionEvaluationResult::Skip);
            }
        };

        let (latest_trigger, trigger_time) = match api.latest_since(&self.target, start_of_range).await? {
            Some(dp) => (dp.value, dp.timestamp),
            None => {
                tracing::trace!("No user-trigger found, skipping");
                return Ok(ActionEvaluationResult::Skip);
            }
        };

        let commands = into_command(latest_trigger);

        if commands.is_empty() {
            tracing::trace!("Trigger not handled by this action, skipping");
            return Ok(ActionEvaluationResult::Skip);
        } else if commands.len() > 2 {
            tracing::error!(
                "Error: more than 2 action are tried to be scheduled for {}. Skipping",
                self.target
            );
            return Ok(ActionEvaluationResult::Skip);
        }

        let source = self.source();

        for command in &commands {
            let needs_execution = needs_execution_for_one_shot_of_target(command, &source, trigger_time, api).await?;

            if !needs_execution {
                tracing::trace!("User-trigger action skipped due to one-shot conditions");
                return Ok(ActionEvaluationResult::Skip);
            }
        }

        tracing::trace!(?commands, ?source, "User-trigger action(s) ready to be executed");

        Ok(ActionEvaluationResult::ExecuteMulti(commands, source))
    }
}

impl UserTriggerAction {
    fn source(&self) -> CommandSource {
        let source_group = match self.target {
            UserTriggerTarget::Remote(_) => "remote".to_string(),
            UserTriggerTarget::Homekit(_) => "homekit".to_string(),
        };
        CommandSource::User(format!("{}:{}", source_group, self.target))
    }

    async fn default_duration(&self, api: &HomeApi) -> Option<Duration> {
        match self.target {
            UserTriggerTarget::Remote(RemoteTarget::BedroomDoor)
            | UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower) => Some(t!(30 minutes)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower) => Some(t!(15 minutes)),
            UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomTvEnergySaving) => {
                match Powered::LivingRoomTv.current_data_point(api).await {
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
        }
    }
}

fn into_command(trigger: UserTrigger) -> Vec<Command> {
    use crate::home::command::*;

    match trigger {
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

        //TODO why no action?
        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(_)) => vec![],
    }
}

fn homekit_heating_actions(zone: HeatingZone, state: HomekitHeatingState) -> Vec<Command> {
    let target_state = match state {
        HomekitHeatingState::Off => HeatingTargetState::Off,
        HomekitHeatingState::Heat(temperature) => HeatingTargetState::Heat { temperature },
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

#[cfg(test)]
mod tests {
    use crate::home::trigger::*;

    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(
            UserTriggerAction::new(UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower)).to_string(),
            "UserTriggerAction[Homekit[InfraredHeaterPower]]"
        );
    }
}
