use api::command::{Command, CommandExecution, CommandId, CommandSource, CommandTarget};
use support::time::DateTime;

use anyhow::Result;

pub mod planning;
pub mod state;

pub trait CommandAccess<C: CommandId> {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution<C::CommandType>>>;

    async fn get_all_commands(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution<C::CommandType>>>;

    async fn get_latest_command_source(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandSource>>;
}

pub trait CommandExecutor<C: Into<Command>> {
    async fn execute(&self, command: C, source: CommandSource) -> Result<()>;
}
