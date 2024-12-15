use anyhow::Result;
use api::command::{Command, CommandSource, CommandTarget};
use support::{t, time::DateTime};

use crate::{planning::action::Action, port::CommandAccess};

use super::command_state::CommandState;

pub trait ActionExecutionState<API> {
    async fn start_just_triggered(&self, api: &API) -> Result<bool>;
    async fn stop_just_triggered(&self, api: &API) -> Result<bool>;
    async fn is_running(&self, api: &API) -> Result<Option<bool>>;
}

impl<API, A> ActionExecutionState<API> for A
where
    API: CommandAccess<Command>,
    A: Action<API>,
    Command: CommandState<API>,
{
    async fn start_just_triggered(&self, api: &API) -> Result<bool> {
        let source =
            get_last_command_source_since(self.start_command(), t!(30 seconds ago), api).await?;
        Ok(source == Some(self.start_command_source()))
    }

    async fn stop_just_triggered(&self, api: &API) -> Result<bool> {
        let source =
            get_last_command_source_since(self.stop_command(), t!(30 seconds ago), api).await?;
        Ok(source == Some(self.stop_command_source()))
    }

    async fn is_running(&self, api: &API) -> Result<Option<bool>> {
        match self.start_command() {
            Some(command) => {
                let is_running = command.is_running(api).await?;
                let last_action_source =
                    get_last_command_source_since(self.start_command(), t!(48 hours ago), api)
                        .await?;
                let started_by_action = Some(self.start_command_source()) == last_action_source;

                Ok(Some(started_by_action && is_running))
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
