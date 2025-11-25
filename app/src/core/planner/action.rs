use std::{fmt::Display, future::Future, pin::Pin};

use anyhow::Result;

use crate::core::id::ExternalId;
use crate::home::command::Command;

use crate::core::HomeApi;
use crate::home::trigger::UserTriggerId;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Execute(Vec<Command>, ExternalId),
    ExecuteTrigger(Vec<Command>, ExternalId, UserTriggerId),
    Skip,
}

pub trait Action: Display + Send + Sync {
    fn ext_id(&self) -> ExternalId;
    fn evaluate<'a>(
        &'a self,
        api: &'a HomeApi,
    ) -> Pin<Box<dyn Future<Output = Result<ActionEvaluationResult>> + Send + 'a>>;
}
