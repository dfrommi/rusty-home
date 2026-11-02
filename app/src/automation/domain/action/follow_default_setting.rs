use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub enum FollowDefaultSetting {
    GlobalDefault,
}

impl Rule for FollowDefaultSetting {
    fn evaluate(&self, _: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        Ok(RuleResult::ExecuteDefault)
    }
}
