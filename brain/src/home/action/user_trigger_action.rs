use api::{
    command::{Command, CommandSource},
    trigger::{ButtonPress, Homekit, Remote, UserTrigger, UserTriggerTarget},
};
use support::t;

use crate::core::planner::{Action, ActionEvaluationResult};

use super::UserTriggerAccess;

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
    API: UserTriggerAccess,
{
    async fn evaluate(&self, api: &API) -> anyhow::Result<ActionEvaluationResult> {
        let latest_trigger = api.latest_since(&self.target, t!(15 seconds ago)).await?;

        match latest_trigger {
            Some(trigger) => Ok(ActionEvaluationResult::Execute(
                into_command(trigger),
                CommandSource::User(format!(
                    "{}:{}",
                    into_source_group(&self.target),
                    self.target
                )),
            )),
            None => Ok(ActionEvaluationResult::Skip),
        }
    }
}

fn into_source_group(target: &UserTriggerTarget) -> String {
    match target {
        UserTriggerTarget::Remote(_) => "remote".to_string(),
        UserTriggerTarget::Homekit(_) => "homekit".to_string(),
    }
}

fn into_command(trigger: UserTrigger) -> Command {
    use api::command::*;

    match trigger {
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::TopSingle)) => {
            Command::SetPower(SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: true,
            })
        }
        UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::BottomSingle)) => {
            Command::SetPower(SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            })
        }
        UserTrigger::Homekit(Homekit::InfraredHeaterPower(on)) => Command::SetPower(SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: on,
        }),
        UserTrigger::Homekit(Homekit::DehumidifierPower(on)) => Command::SetPower(SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: on,
        }),
        UserTrigger::Homekit(Homekit::LivingRoomTvEnergySaving(on)) => {
            Command::SetEnergySaving(SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
                on,
            })
        }
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
