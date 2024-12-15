use anyhow::Result;
use api::command::{Command, CommandSource};
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
        let source = get_last_command_source_since(self, t!(30 seconds ago), api).await?;
        Ok(source == Some(self.start_command_source()))
    }

    async fn stop_just_triggered(&self, api: &API) -> Result<bool> {
        let source = get_last_command_source_since(self, t!(30 seconds ago), api).await?;
        Ok(source == Some(self.start_command_source()))
    }

    async fn is_running(&self, api: &API) -> Result<Option<bool>> {
        if self.start_just_triggered(api).await? {
            return Ok(Some(true));
        } else if self.stop_just_triggered(api).await? {
            return Ok(Some(false));
        }

        match self.start_command() {
            Some(command) => Ok(Some(command.is_running(api).await?)),
            None => Ok(None),
        }
    }
}

async fn get_last_command_source_since<T>(
    action: &impl Action<T>,
    since: DateTime,
    api: &T,
) -> Result<Option<CommandSource>>
where
    T: CommandAccess<Command>,
{
    api.get_latest_command_source(action.controls_target(), since)
        .await
}
