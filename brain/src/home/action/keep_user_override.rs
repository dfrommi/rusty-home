use std::fmt::Display;

use crate::{
    core::planner::{Action, ActionEvaluationResult},
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

impl Action for KeepUserOverride {
    async fn evaluate(&self, api: &crate::Database) -> Result<ActionEvaluationResult> {
        let fulfilled = api.current(self.user_controlled.clone()).await?;

        if fulfilled {
            Ok(ActionEvaluationResult::Lock(self.target.clone()))
        } else {
            Ok(ActionEvaluationResult::Skip)
        }
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.user_controlled)
    }
}
