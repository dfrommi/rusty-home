use std::fmt::Display;

use anyhow::Result;

use crate::core::id::ExternalId;
use crate::home::command::{Command, CommandSource, CommandTarget};

use crate::core::HomeApi;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Lock(CommandTarget),
    Execute(Command, CommandSource),
    ExecuteMulti(Vec<Command>, CommandSource),
    Skip,
}

pub trait Action: Display {
    fn ext_id(&self) -> ExternalId;
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult>;
}
