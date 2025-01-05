use api::{
    command::{Command, CommandSource},
    state::Powered,
    trigger::{
        ButtonPress, Homekit, HomekitTarget, Remote, RemoteTarget, UserTrigger, UserTriggerTarget,
    },
};
use support::{t, time::DateTime};

use crate::core::{
    planner::{Action, ActionEvaluationResult},
    service::CommandState,
};

use super::{trigger_once_and_keep_running, CommandAccess, DataPointAccess, UserTriggerAccess};

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

impl<API> Action<API> for UserTriggerAction
where
    API: UserTriggerAccess + CommandAccess + CommandState + DataPointAccess<Powered>,
{
    async fn evaluate(&self, api: &API) -> anyhow::Result<ActionEvaluationResult> {
        let start_of_range = match self.range_start(api).await {
            Some(duration) => duration,
            None => return Ok(ActionEvaluationResult::Skip),
        };

        let latest_trigger = api.latest_since(&self.target, start_of_range).await?;

        let command = match latest_trigger.and_then(into_command) {
            Some(c) => c.into(),
            None => return Ok(ActionEvaluationResult::Skip),
        };

        let source = self.source();

        let fulfilled =
            trigger_once_and_keep_running(&command, &source, start_of_range, api).await?;

        if !fulfilled {
            return Ok(ActionEvaluationResult::Skip);
        }

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

    async fn range_start<API>(&self, api: &API) -> Option<DateTime>
    where
        API: DataPointAccess<Powered>,
    {
        match self.target {
            UserTriggerTarget::Remote(RemoteTarget::BedroomDoor)
            | UserTriggerTarget::Homekit(HomekitTarget::InfraredHeaterPower) => {
                Some(t!(30 minutes ago))
            }
            UserTriggerTarget::Homekit(HomekitTarget::DehumidifierPower) => {
                Some(t!(15 minutes ago))
            }
            UserTriggerTarget::Homekit(HomekitTarget::LivingRoomTvEnergySaving) => {
                match api.current_data_point(Powered::LivingRoomTv).await {
                    Ok(dp) if dp.value => Some(dp.timestamp),
                    Ok(_) => None,
                    Err(e) => {
                        tracing::error!("Error getting current state of living room tv: {:?}", e);
                        None
                    }
                }
            }
        }
    }
}

fn into_command(trigger: UserTrigger) -> Option<Command> {
    use api::command::*;

    match trigger {
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::TopSingle)) => {
            Some(Command::SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: true,
            })
        }
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::BottomSingle)) => {
            Some(Command::SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            })
        }
        UserTrigger::Homekit(Homekit::InfraredHeaterPower(on)) => Some(Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }),
        UserTrigger::Homekit(Homekit::DehumidifierPower(on)) => Some(Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }),
        UserTrigger::Homekit(Homekit::LivingRoomTvEnergySaving(on)) if !on => {
            Some(Command::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
                on: false,
            })
        }

        UserTrigger::Homekit(Homekit::LivingRoomTvEnergySaving(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use api::trigger::*;

    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(
            UserTriggerAction::new(UserTriggerTarget::Homekit(
                HomekitTarget::InfraredHeaterPower
            ))
            .to_string(),
            "UserTriggerAction[Homekit[InfraredHeaterPower]]"
        );
    }
}
