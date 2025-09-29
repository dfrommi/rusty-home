use crate::adapter::homekit::{HomekitCommand, HomekitCommandTarget, HomekitHeatingState};
use crate::core::HomeApi;
use crate::core::planner::{Action, ActionEvaluationResult};
use crate::core::time::Duration;
use crate::home::command::{Command, CommandSource, HeatingTargetState, Thermostat};
use crate::home::state::Powered;
use crate::home::trigger::{ButtonPress, Remote, RemoteTarget, UserTrigger, UserTriggerTarget};
use crate::t;

use super::{DataPointAccess, trigger_once_and_keep_running};

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

        let command = match into_command(latest_trigger) {
            Some(c) => c,
            None => {
                tracing::trace!("Trigger not handled by this action, skipping");
                return Ok(ActionEvaluationResult::Skip);
            }
        };

        let source = self.source();

        let fulfilled = trigger_once_and_keep_running(&command, &source, trigger_time, api).await?;

        if !fulfilled {
            tracing::trace!("User-trigger action skipped due to one-shot conditions");
            return Ok(ActionEvaluationResult::Skip);
        }

        tracing::trace!(?command, ?source, "User-trigger action ready to be executed");

        Ok(ActionEvaluationResult::Execute(command, source))
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

fn into_command(trigger: UserTrigger) -> Option<Command> {
    use crate::home::command::*;

    match trigger {
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::TopSingle)) => Some(Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: true,
        }),
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::BottomSingle)) => Some(Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: false,
        }),
        UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(on)) => Some(Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }),
        UserTrigger::Homekit(HomekitCommand::DehumidifierPower(on)) => Some(Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }),
        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(on)) if !on => Some(Command::SetEnergySaving {
            device: EnergySavingDevice::LivingRoomTv,
            on: false,
        }),
        UserTrigger::Homekit(HomekitCommand::LivingRoomCeilingFanSpeed(speed)) => Some(Command::ControlFan {
            device: Fan::LivingRoomCeilingFan,
            speed,
        }),
        UserTrigger::Homekit(HomekitCommand::BedroomCeilingFanSpeed(speed)) => Some(Command::ControlFan {
            device: Fan::BedroomCeilingFan,
            speed,
        }),
        UserTrigger::Homekit(HomekitCommand::LivingRoomHeatingState(state)) => {
            homekit_heating_action(Thermostat::LivingRoom, state)
        }
        UserTrigger::Homekit(HomekitCommand::BedroomHeatingState(state)) => {
            homekit_heating_action(Thermostat::Bedroom, state)
        }
        UserTrigger::Homekit(HomekitCommand::KitchenHeatingState(state)) => {
            homekit_heating_action(Thermostat::Kitchen, state)
        }
        UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(state)) => {
            homekit_heating_action(Thermostat::RoomOfRequirements, state)
        }

        UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(_)) => None,
    }
}

fn homekit_heating_action(thermostat: Thermostat, state: HomekitHeatingState) -> Option<Command> {
    Some(Command::SetHeating {
        device: thermostat,
        target_state: match state {
            HomekitHeatingState::Off => HeatingTargetState::Off,
            HomekitHeatingState::Heat(temperature) => HeatingTargetState::Heat {
                temperature,
                duration: t!(1 hours),
            },
            HomekitHeatingState::Auto => return None,
        },
    })
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
