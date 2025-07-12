use std::fmt::Display;

use crate::{
    core::HomeApi,
    core::planner::{Action, ActionEvaluationResult},
    home::state::UserControlled,
};

use super::DataPointAccess;
use crate::home::command::CommandTarget;
use anyhow::Result;

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
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult> {
        let fulfilled = self.user_controlled.current(api).await?;

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
