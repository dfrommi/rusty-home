use std::fmt::Display;

use api::command::{Command, CommandTarget};

use crate::core::planner::{CommandAction, ConditionalAction};

#[derive(Debug, Clone)]
pub struct FollowDefaultSetting {
    target: CommandTarget,
}

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self { target }
    }
}

impl Display for FollowDefaultSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FollowDefaultSetting[{}]", self.target)
    }
}

impl ConditionalAction<()> for FollowDefaultSetting {
    async fn preconditions_fulfilled(&self, _: &()) -> anyhow::Result<bool> {
        Ok(true)
    }
}

impl CommandAction for FollowDefaultSetting {
    fn command(&self) -> Command {
        match self.target.clone() {
            CommandTarget::SetPower { device } => Command::SetPower(api::command::SetPower {
                device,
                power_on: false,
            }),
            CommandTarget::SetHeating { device } => Command::SetHeating(api::command::SetHeating {
                device,
                target_state: api::command::HeatingTargetState::Auto,
            }),
            CommandTarget::PushNotify {
                recipient,
                notification,
            } => Command::PushNotify(api::command::PushNotify {
                action: api::command::NotificationAction::Dismiss,
                notification,
                recipient,
            }),
            CommandTarget::SetEnergySaving { device } => {
                Command::SetEnergySaving(api::command::SetEnergySaving { device, on: true })
            }
        }
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}
