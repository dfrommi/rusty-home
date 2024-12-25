use std::fmt::Display;

use anyhow::Result;

use api::command::{Command, CommandSource, CommandTarget};

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Lock(CommandTarget),
    Execute(Command, CommandSource),
    Skip,
}

pub trait Action<API>: Display {
    async fn evaluate(&self, api: &API) -> Result<ActionEvaluationResult>;
}

pub trait ConditionalAction<API> {
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool>;
}

pub trait CommandAction {
    fn command(&self) -> Command;
    fn source(&self) -> CommandSource;
}

impl<T: CommandAction + ConditionalAction<API> + Display, API> Action<API> for T {
    async fn evaluate(&self, api: &API) -> Result<ActionEvaluationResult> {
        let preconditions_fulfilled = self.preconditions_fulfilled(api).await?;

        if !preconditions_fulfilled {
            return Ok(ActionEvaluationResult::Skip);
        }

        Ok(ActionEvaluationResult::Execute(
            self.command(),
            self.source(),
        ))
    }
}
