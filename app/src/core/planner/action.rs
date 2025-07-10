use std::fmt::Display;

use anyhow::Result;

use crate::home::command::{Command, CommandSource, CommandTarget};

use crate::core::HomeApi;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Lock(CommandTarget),
    Execute(Command, CommandSource),
    Skip,
}

pub trait Action: Display {
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult>;
}

pub trait SimpleAction: Display {
    fn command(&self) -> Command;
    fn source(&self) -> CommandSource;
    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool>;
}

impl<T: SimpleAction> Action for T {
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult> {
        let preconditions_fulfilled = self.preconditions_fulfilled(api).await?;

        if !preconditions_fulfilled {
            return Ok(ActionEvaluationResult::Skip);
        }

        Ok(ActionEvaluationResult::Execute(self.command(), self.source()))
    }
}
