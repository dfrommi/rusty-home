use std::fmt::Display;

use anyhow::Result;

use crate::core::id::ExternalId;
use crate::home::command::Command;

use crate::home::RuleEvaluationContext;
use crate::trigger::UserTriggerId;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Execute(Vec<Command>, ExternalId),
    ExecuteTrigger(Vec<Command>, ExternalId, UserTriggerId),
    Skip,
}

pub trait Action: Display + Send + Sync {
    fn ext_id(&self) -> ExternalId;
    fn evaluate<'a>(&'a self, ctx: &'a RuleEvaluationContext) -> Result<ActionEvaluationResult>;
}
