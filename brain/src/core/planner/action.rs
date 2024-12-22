use anyhow::Result;

use api::command::{Command, CommandSource, CommandTarget};
use support::{t, time::DateTime};

use crate::port::{CommandAccess, CommandExecutionResult, CommandExecutor};

use super::{CommandState, Lockable};

pub trait ConditionalAction<API> {
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool>;
}

pub trait CommandAction {
    fn command(&self) -> Command;
    fn source(&self) -> CommandSource;
}

pub trait ExecutableAction<E> {
    async fn execute(&self, executor: &E) -> Result<CommandExecutionResult>;
}

pub trait ExecutionAwareAction<API> {
    async fn was_latest_execution_for_target_since(
        &self,
        since: DateTime,
        api: &API,
    ) -> Result<bool>;

    async fn is_reflected_in_state(&self, api: &API) -> Result<bool>;
}

impl<T: CommandAction> Lockable<CommandTarget> for T {
    fn locking_key(&self) -> CommandTarget {
        let command = self.command();
        command.into()
    }
}

impl<E, T> ExecutableAction<E> for T
where
    E: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
    T: CommandAction + ExecutionAwareAction<E>,
{
    async fn execute(&self, executor: &E) -> Result<CommandExecutionResult> {
        let was_latest_execution = self
            .was_latest_execution_for_target_since(t!(48 hours ago), executor)
            .await?;
        let is_reflected_in_state = self.is_reflected_in_state(executor).await?;

        if !was_latest_execution || !is_reflected_in_state {
            executor.execute(self.command(), self.source()).await?;
            Ok(CommandExecutionResult::Triggered)
        } else {
            Ok(CommandExecutionResult::Skipped)
        }
    }
}

impl<T, API> ExecutionAwareAction<API> for T
where
    T: CommandAction,
    API: CommandState<Command> + CommandAccess<Command>,
{
    async fn was_latest_execution_for_target_since(
        &self,
        since: DateTime,
        api: &API,
    ) -> Result<bool> {
        let command = self.command();
        let target = CommandTarget::from(&command);
        let source = self.source();

        let latest_source = api.get_latest_command_source(target, since).await?;

        Ok(latest_source.map(|s| s == source).unwrap_or(false))
    }

    async fn is_reflected_in_state(&self, api: &API) -> Result<bool> {
        let command = self.command();
        let is_reflected_in_state = api.is_reflected_in_state(&command).await?;

        Ok(is_reflected_in_state)
    }
}
