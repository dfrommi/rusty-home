use api::command::{Command, CommandTarget};
use support::t;

use crate::port::{CommandAccess, CommandExecutionResult, CommandExecutor, CommandStore};

use super::CommandState;

impl<T> CommandExecutor for T
where
    T: CommandStore + CommandState + CommandAccess,
{
    async fn execute(
        &self,
        command: Command,
        source: api::command::CommandSource,
    ) -> anyhow::Result<CommandExecutionResult> {
        let target: CommandTarget = command.clone().into();
        let last_execution = self
            .get_latest_command(target, t!(48 hours ago))
            .await?
            .filter(|e| e.source == source && e.command == command)
            .map(|e| e.created);

        //wait until roundtrip is completed. State might not have been updated yet
        let was_just_executed = last_execution.map_or(false, |dt| dt > t!(30 seconds ago));

        if was_just_executed {
            return Ok(CommandExecutionResult::Skipped);
        }

        let was_latest_execution = last_execution.is_some();
        let is_reflected_in_state = self.is_reflected_in_state(&command).await?;

        if !was_latest_execution || !is_reflected_in_state {
            self.save_command(command, source).await?;
            Ok(CommandExecutionResult::Triggered)
        } else {
            Ok(CommandExecutionResult::Skipped)
        }
    }
}
