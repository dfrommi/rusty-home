use api::command::{Command, CommandSource, CommandTarget};
use support::{ext::ResultExt, t, time::DateTime};

use crate::thing::{planning::action::Action, CommandAccess};

pub struct ExecutionAwareAction<T, A>
where
    T: CommandAccess<Command>,
    A: Action<T>,
{
    action: A,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, A> Action<T> for ExecutionAwareAction<T, A>
where
    T: CommandAccess<Command>,
    A: Action<T>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> anyhow::Result<bool> {
        if self.just_started(api).await {
            return Ok(true);
        } else if self.just_stopped(api).await {
            return Ok(false);
        }

        self.action.preconditions_fulfilled(api).await
    }

    async fn is_running(&self, api: &T) -> anyhow::Result<bool> {
        if self.just_started(api).await {
            return Ok(true);
        } else if self.just_stopped(api).await {
            return Ok(false);
        }

        self.action.is_running(api).await
    }

    fn start_command(&self) -> Option<Command> {
        self.action.start_command()
    }

    fn start_command_source(&self) -> CommandSource {
        self.action.start_command_source()
    }

    fn stop_command(&self) -> Option<Command> {
        self.action.stop_command()
    }

    fn stop_command_source(&self) -> CommandSource {
        self.action.stop_command_source()
    }

    fn controls_target(&self) -> Option<CommandTarget> {
        self.action.controls_target()
    }
}

impl<T, A> std::fmt::Display for ExecutionAwareAction<T, A>
where
    T: CommandAccess<Command>,
    A: Action<T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.action)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandSourceType {
    Start,
    Stop,
}

impl<T, A> ExecutionAwareAction<T, A>
where
    T: CommandAccess<Command>,
    A: Action<T>,
{
    pub fn new(action: A) -> Self {
        Self {
            action,
            _phantom: std::marker::PhantomData,
        }
    }

    async fn just_started(&self, api: &T) -> bool
    where
        T: CommandAccess<Command>,
    {
        get_last_command_source_since(self, t!(30 seconds ago), api).await
            == Some(self.start_command_source())
    }

    async fn just_stopped(&self, api: &T) -> bool
    where
        T: CommandAccess<Command>,
    {
        get_last_command_source_since(self, t!(30 seconds ago), api).await
            == Some(self.start_command_source())
    }
}

async fn get_last_command_source_since<T>(
    action: &impl Action<T>,
    since: DateTime,
    api: &T,
) -> Option<CommandSource>
where
    T: CommandAccess<Command>,
{
    if let Some(target) = action.controls_target() {
        api.get_latest_command_source(target, since)
            .await
            .unwrap_or_warn(
                None,
                format!("Error getting last command type of {}", action).as_str(),
            )
    } else {
        None
    }
}
