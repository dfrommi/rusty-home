use api::command::{Command, CommandTarget};
use support::t;

use crate::port::{CommandAccess, CommandExecutionResult, CommandExecutor, CommandStore};

use super::CommandState;

impl<T> CommandExecutor for T
where
    T: CommandStore + CommandState<Command> + CommandAccess<Command>,
{
    async fn execute(
        &self,
        command: Command,
        source: api::command::CommandSource,
    ) -> anyhow::Result<CommandExecutionResult> {
        let target: CommandTarget = command.clone().into();
        let last_execution = self.get_latest_command(target, t!(48 hours ago)).await?;

        //wait until roundtrip is completed. State might not have been updated yet
        let was_just_executed = last_execution.as_ref().map_or(false, |e| {
            e.created > t!(30 seconds ago) && e.source == source
        });

        if was_just_executed {
            return Ok(CommandExecutionResult::Skipped);
        }

        let was_latest_execution = last_execution.map_or(false, |e| e.source == source);
        let is_reflected_in_state = self.is_reflected_in_state(&command).await?;

        if !was_latest_execution || !is_reflected_in_state {
            self.save_command(command, source).await?;
            Ok(CommandExecutionResult::Triggered)
        } else {
            Ok(CommandExecutionResult::Skipped)
        }
    }
}
