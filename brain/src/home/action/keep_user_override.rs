use std::fmt::Display;

use crate::{
    core::planner::{ConditionalAction, ExecutableAction, ExecutionAwareAction, Lockable},
    home::state::UserControlled,
};

use super::DataPointAccess;
use anyhow::Result;
use api::command::CommandTarget;

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

impl<T> ConditionalAction<T> for KeepUserOverride
where
    T: DataPointAccess<UserControlled>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(self.user_controlled.clone()).await
    }
}

impl Lockable<CommandTarget> for KeepUserOverride {
    fn locking_key(&self) -> CommandTarget {
        self.target.clone()
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.user_controlled)
    }
}

impl ExecutionAwareAction<()> for KeepUserOverride {
    async fn was_latest_execution_for_target_since(
        &self,
        _: support::time::DateTime,
        _: &(),
    ) -> Result<bool> {
        Ok(false)
    }

    async fn is_reflected_in_state(&self, _: &()) -> Result<bool> {
        Ok(false)
    }
}

impl ExecutableAction<()> for KeepUserOverride {
    async fn execute(&self, _: &()) -> Result<super::CommandExecutionResult> {
        Ok(super::CommandExecutionResult::Skipped)
    }
}
