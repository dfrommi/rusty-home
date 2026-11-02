use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::CommandTarget;

#[derive(Debug, Clone, Id)]
pub struct FollowDefaultSetting(CommandTarget);

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self(target)
    }
}

impl Rule for FollowDefaultSetting {
    fn evaluate(&self, _: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        Ok(RuleResult::ExecuteDefault)
    }
}
