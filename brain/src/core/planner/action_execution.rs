use anyhow::{Context, Result};
use api::command::{Command, CommandSource, CommandTarget};
use support::{t, time::DateTime};

use crate::port::{CommandAccess, CommandExecutor};

use super::command_state::CommandState;

#[derive(Debug, Clone)]
pub struct ActionExecution<C> {
    action_name: String,
    start_command: Option<C>,
    start_source: CommandSource,
    stop_command: Option<C>,
    stop_source: CommandSource,
    controlled_target: CommandTarget,
}

//CONSTRUCTORS
impl<C> ActionExecution<C> {
    pub fn from_start(action_name: String, start_command: C) -> Self
    where
        for<'a> &'a C: Into<CommandTarget>,
    {
        let target: CommandTarget = (&start_command).into();
        Self::from(action_name, Some(start_command), None, target)
    }

    pub fn from_start_and_stop(action_name: String, start_command: C, stop_command: C) -> Self
    where
        for<'a> &'a C: Into<CommandTarget>,
    {
        let start_target: CommandTarget = (&start_command).into();
        let stop_target: CommandTarget = (&stop_command).into();

        if start_target != stop_target {
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

    pub fn locking_only(action_name: String, target: CommandTarget) -> Self {
        Self::from(action_name, None, None, target)
    }

    fn from(
        action_name: String,
        start_command: Option<C>,
        stop_command: Option<C>,
        controlled_target: CommandTarget,
    ) -> Self {
        Self {
            action_name: action_name.to_string(),
            controlled_target,
            start_command,
            start_source: CommandSource::System(format!("planning:{}:start", action_name)),
            stop_command,
            stop_source: CommandSource::System(format!("planning:{}:stop", action_name)),
        }
    }

    pub fn into(self) -> ActionExecution<Command>
    where
        C: Into<Command>,
    {
        ActionExecution {
            action_name: self.action_name,
            controlled_target: self.controlled_target,
            start_command: self.start_command.map(|c| c.into()),
            start_source: self.start_source,
            stop_command: self.stop_command.map(|c| c.into()),
            stop_source: self.stop_source,
        }
    }
}

//ACCESSORS
impl<C> ActionExecution<C> {
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

impl<C: Into<Command>> ActionExecution<C> {
    pub async fn latest_trigger_since(
        &self,
        api: &impl CommandAccess<C>,
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
        api: &impl CommandAccess<C>,
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
        api: &(impl CommandAccess<C> + CommandState<C>),
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

    pub async fn is_reflected_in_state(&self, api: &impl CommandState<C>) -> Result<bool> {
        match &self.start_command {
            Some(command) => api.is_reflected_in_state(command).await,
            None => Ok(false),
        }
    }

    pub async fn execute_start(&self, executor: &impl CommandExecutor<C>) -> Result<()>
    where
        C: Into<Command> + Clone + std::fmt::Debug,
    {
        match &self.start_command {
            Some(command) => {
                tracing::debug!("Executing start command {:?} via action {}", command, self);
                executor
                    .execute(command.clone(), self.start_source.clone())
                    .await
                    .with_context(|| {
                        format!("Error executing command {:?} via action {}", command, self)
                    })
            }
            None => anyhow::bail!(
                "Action {} should be started, but no command is configured",
                self
            ),
        }
    }

    pub async fn execute_stop(&self, executor: &impl CommandExecutor<C>) -> Result<()>
    where
        C: Into<Command> + Clone + std::fmt::Debug,
    {
        match &self.stop_command {
            Some(command) => {
                tracing::debug!("Executing stop command {:?} via action {}", command, self);
                executor
                    .execute(command.clone(), self.stop_source.clone())
                    .await
                    .with_context(|| {
                        format!("Error executing command {:?} via action {}", command, self)
                    })
            }
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

impl<C> std::fmt::Display for ActionExecution<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.action_name)
    }
}
