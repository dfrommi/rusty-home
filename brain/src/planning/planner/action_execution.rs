use anyhow::{Context, Result};
use api::command::{Command, CommandSource, CommandTarget};
use support::{t, time::DateTime};

use crate::port::{CommandAccess, CommandExecutor};

use super::command_state::CommandState;

#[derive(Debug, Clone)]
pub struct ActionExecution {
    action_name: String,
    start_command: Option<Command>,
    start_source: CommandSource,
    stop_command: Option<Command>,
    stop_source: CommandSource,
    controlled_target: CommandTarget,
}

//CONSTRUCTORS
impl ActionExecution {
    pub fn from_start(action_name: &str, start_command: impl Into<Command>) -> Self {
        let start_command = start_command.into();
        let target = CommandTarget::from(&start_command);
        Self::from(action_name, Some(start_command), None, target)
    }

    pub fn from_start_and_stop(
        action_name: &str,
        start_command: impl Into<Command>,
        stop_command: impl Into<Command>,
    ) -> Self {
        let start_command = start_command.into();
        let stop_command = stop_command.into();

        let start_target = CommandTarget::from(&start_command);
        if start_target != CommandTarget::from(&stop_command) {
            tracing::error!(
                "Action {} controls different devices in start and stop commands. Falling back to start command",
                action_name
            );
        }

        Self::from(
            action_name,
            Some(start_command),
            Some(stop_command),
            start_target,
        )
    }

    pub fn locking_only(action_name: &str, target: CommandTarget) -> Self {
        Self::from(action_name, None, None, target)
    }

    fn from(
        action_name: &str,
        start_command: Option<Command>,
        stop_command: Option<Command>,
        controlled_target: CommandTarget,
    ) -> Self {
        Self {
            action_name: action_name.to_owned(),
            controlled_target,
            start_command,
            start_source: CommandSource::System(format!("planning:{}:start", action_name)),
            stop_command,
            stop_source: CommandSource::System(format!("planning:{}:stop", action_name)),
        }
    }
}

//ACCESSORS
impl ActionExecution {
    pub fn controlled_target(&self) -> &CommandTarget {
        &self.controlled_target
    }

    pub fn can_be_started(&self) -> bool {
        self.start_command.is_some()
    }

    pub fn can_be_stopped(&self) -> bool {
        self.stop_command.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionExecutionTrigger {
    Start,
    Stop,
    Other,
    None,
}

impl ActionExecution {
    pub async fn latest_trigger_since(
        &self,
        api: &impl CommandAccess<Command>,
        since: DateTime,
    ) -> Result<ActionExecutionTrigger> {
        let source = api
            .get_latest_command_source(self.controlled_target.clone(), since)
            .await?;
        match source {
            Some(source) => Ok(self.to_trigger(&source)),
            None => Ok(ActionExecutionTrigger::None),
        }
    }

    pub async fn any_trigger_since(
        &self,
        api: &impl CommandAccess<Command>,
        trigger: ActionExecutionTrigger,
        since: DateTime,
    ) -> Result<bool> {
        let result = api
            .get_all_commands(self.controlled_target.clone(), since)
            .await?
            .iter()
            .any(|c| self.to_trigger(&c.source) == trigger);

        Ok(result)
    }

    pub async fn action_started_and_still_reflected(
        &self,
        api: &(impl CommandAccess<Command> + CommandState<Command>),
    ) -> Result<Option<bool>> {
        match &self.start_command {
            Some(command) => {
                let is_running = api.is_reflected_in_state(command).await?;
                let last_trigger = self.latest_trigger_since(api, t!(48 hours ago)).await?;

                Ok(Some(
                    last_trigger == ActionExecutionTrigger::Start && is_running,
                ))
            }
            None => Ok(None),
        }
    }

    pub async fn execute_start(&self, executor: &impl CommandExecutor<Command>) -> Result<()> {
        match &self.start_command {
            Some(command) => executor
                .execute(command.clone(), self.start_source.clone())
                .await
                .with_context(|| {
                    format!("Error executing command {:?} via action {}", command, self)
                }),
            None => anyhow::bail!(
                "Action {} should be started, but no command is configured",
                self
            ),
        }
    }

    pub async fn execute_stop(&self, executor: &impl CommandExecutor<Command>) -> Result<()> {
        match &self.stop_command {
            Some(command) => executor
                .execute(command.clone(), self.stop_source.clone())
                .await
                .with_context(|| {
                    format!("Error executing command {:?} via action {}", command, self)
                }),
            None => anyhow::bail!(
                "Action {} should be stopped, but no command is configured",
                self
            ),
        }
    }

    fn to_trigger(&self, source: &CommandSource) -> ActionExecutionTrigger {
        if source == &self.start_source {
            ActionExecutionTrigger::Start
        } else if source == &self.stop_source {
            ActionExecutionTrigger::Stop
        } else {
            ActionExecutionTrigger::Other
        }
    }
}

impl std::fmt::Display for ActionExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.action_name)
    }
}
