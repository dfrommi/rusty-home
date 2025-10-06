use std::{fmt::Display, future::Future, pin::Pin};

use anyhow::Result;

use crate::core::id::ExternalId;
use crate::home::command::{Command, CommandTarget};

use crate::core::HomeApi;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Lock(CommandTarget),
    Execute(Command, ExternalId),
    ExecuteMulti(Vec<Command>, ExternalId),
    Skip,
}

pub trait Action: Display + Send + Sync {
    fn ext_id(&self) -> ExternalId;
    fn evaluate<'a>(
        &'a self,
        api: &'a HomeApi,
    ) -> Pin<Box<dyn Future<Output = Result<ActionEvaluationResult>> + Send + 'a>>;
}
