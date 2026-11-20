use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::home_state::{Presence, Resident};
use crate::t;

#[derive(Debug, Clone, Id)]
pub enum BlockAutomation {
    AtNight,
    WhenPresenceInLivingRoom,
}

impl Rule for BlockAutomation {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let should_block = match self {
            BlockAutomation::AtNight => is_at_night(ctx.current(Resident::AnyoneSleeping)?),
            BlockAutomation::WhenPresenceInLivingRoom => is_presence(ctx)?,
        };

        if should_block {
            Ok(RuleResult::ExecuteDefault)
        } else {
            Ok(RuleResult::Skip)
        }
    }
}

fn is_at_night(sleeping: bool) -> bool {
    let nighttime = t!(22:00 - 9:00).active().is_some();

    if !sleeping && !nighttime {
        tracing::info!("Not sleeping and not nighttime; not blocking automation");
        return false;
    }

    tracing::info!("Sleep mode or nighttime active; blocking automation");
    true
}

fn is_presence(ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
    if ctx.current(Presence::RecentInLivingRoom)? {
        tracing::info!("Blocking automation because someone was recently in the living room");
        Ok(true)
    } else {
        Ok(false)
    }
}
