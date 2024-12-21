use std::fmt::Display;

use crate::home::state::UserControlled;

use super::{Action, ActionExecution, DataPointAccess};
use anyhow::Result;
use api::command::{Command, CommandTarget};

#[derive(Debug, Clone)]
pub struct KeepUserOverride {
    user_controlled: UserControlled,
    target: CommandTarget,
}

impl KeepUserOverride {
    pub fn new(user_controlled: UserControlled, target: CommandTarget) -> Self {
        Self {
            user_controlled,
            target,
        }
    }
}

impl<T> Action<T, Command> for KeepUserOverride
where
    T: DataPointAccess<UserControlled>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(self.user_controlled.clone()).await
    }

    fn execution(&self) -> ActionExecution<Command> {
        ActionExecution::locking(self.to_string(), self.target.clone())
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.user_controlled)
    }
}
