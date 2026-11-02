use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::home_state::Resident;
use crate::t;

#[derive(Debug, Clone, Id)]
pub enum BlockAutomation {
    AtNight,
}

impl Rule for BlockAutomation {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let sleeping = ctx.current(Resident::AnyoneSleeping)?;
        let nighttime = t!(22:00 - 9:00).active().is_some();

        if !sleeping && !nighttime {
            tracing::info!("Not sleeping and not nighttime; not blocking automation");
            return Ok(RuleResult::Skip);
        }

        tracing::info!("Sleep mode or nighttime active; blocking automation");
        Ok(RuleResult::ExecuteDefault)
    }
}
