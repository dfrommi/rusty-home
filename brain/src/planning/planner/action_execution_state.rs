use anyhow::Result;
use api::command::{Command, CommandSource, CommandTarget};
use support::{t, time::DateTime};

use crate::{planning::action::Action, port::CommandAccess};

use super::command_state::CommandState;

pub trait ActionExecutionState<API> {
    async fn start_latest_trigger_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>;

    async fn stop_latest_trigger_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>;

    async fn was_started_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>;

    async fn is_running(&self, api: &API) -> Result<Option<bool>>
    where
        API: CommandAccess<Command>,
        Command: CommandState<API>;
}

impl<API, A> ActionExecutionState<API> for A
where
    A: Action<API>,
{
    async fn start_latest_trigger_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>,
    {
        let source = get_last_command_source_since(self.start_command(), since, api).await?;
        Ok(source == Some(self.start_command_source()))
    }

    async fn was_started_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>,
    {
        let source = self.start_command_source();

        let result = match self.start_command() {
            Some(command) => api
                .get_all_commands(CommandTarget::from(command), since)
                .await?
                .iter()
                .any(|c| c.source == source),
            None => false,
        };

        Ok(result)
    }

    async fn stop_latest_trigger_since(&self, api: &API, since: DateTime) -> Result<bool>
    where
        API: CommandAccess<Command>,
    {
        let source = get_last_command_source_since(self.stop_command(), since, api).await?;
        Ok(source == Some(self.stop_command_source()))
    }

    async fn is_running(&self, api: &API) -> Result<Option<bool>>
    where
        API: CommandAccess<Command>,
        Command: CommandState<API>,
    {
        match self.start_command() {
            Some(command) => {
                let is_running = command.is_running(api).await?;
                let last_triggered_by_start = self
                    .start_latest_trigger_since(api, t!(48 hours ago))
                    .await?;

                Ok(Some(last_triggered_by_start && is_running))
            }
            None => Ok(None),
        }
    }
}

async fn get_last_command_source_since<T>(
    command: Option<Command>,
    since: DateTime,
    api: &T,
) -> Result<Option<CommandSource>>
where
    T: CommandAccess<Command>,
{
    Ok(match command {
        Some(command) => {
            api.get_latest_command_source(CommandTarget::from(command), since)
                .await?
        }

        None => None,
    })
}
