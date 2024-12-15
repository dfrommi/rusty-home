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

    fn start_command(&self) -> Option<Command> {
        None
    }

    fn stop_command(&self) -> Option<Command> {
        None
    }

    fn controls_target(&self) -> CommandTarget {
        self.1.clone()
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.0)
    }
}
