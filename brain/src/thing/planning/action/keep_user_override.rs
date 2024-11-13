use std::fmt::Display;

use super::{Action, DataPointAccess, UserControlled};
use anyhow::Result;
use api::command::{Command, CommandTarget};

#[derive(Debug, Clone)]
pub struct KeepUserOverride(UserControlled, CommandTarget);

impl KeepUserOverride {
    pub fn new(user_controlled: UserControlled, target: CommandTarget) -> Self {
        Self(user_controlled, target)
    }
}

impl<T> Action<T> for KeepUserOverride
where
    T: DataPointAccess<UserControlled>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(self.0.clone()).await
    }

    //TODO avoid duplicate call and potential issue around time gap
    async fn is_running(&self, api: &T) -> Result<bool> {
        self.preconditions_fulfilled(api).await
    }

    fn start_command(&self) -> Option<Command> {
        tracing::warn!("User controlled action {} should never be started", self);
        None
    }

    fn stop_command(&self) -> Option<Command> {
        tracing::warn!("User controlled action {} should never be stopped", self);
        None
    }

    fn controls_target(&self) -> Option<CommandTarget> {
        Some(self.1.clone())
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.0)
    }
}
