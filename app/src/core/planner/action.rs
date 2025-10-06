use std::fmt::Display;

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

pub trait Action: Display {
    fn ext_id(&self) -> ExternalId;
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult>;
}
