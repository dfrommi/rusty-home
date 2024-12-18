use std::fmt::Display;

use crate::planning::planner::ActionExecution;

use super::{Action, DataPointAccess, UserControlled};
use anyhow::Result;
use api::command::CommandTarget;

#[derive(Debug, Clone)]
pub struct KeepUserOverride {
    user_controlled: UserControlled,
    execution: ActionExecution,
}

impl KeepUserOverride {
    pub fn new(user_controlled: UserControlled, target: CommandTarget) -> Self {
        let action_name = format!("KeepUserOverride[{}]", &user_controlled);
        Self {
            user_controlled,
            execution: ActionExecution::locking_only(action_name.as_str(), target),
        }
    }
}

impl<T> Action<T> for KeepUserOverride
where
    T: DataPointAccess<UserControlled>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(self.user_controlled.clone()).await
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.user_controlled)
    }
}
