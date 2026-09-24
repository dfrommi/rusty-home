use std::collections::HashMap;

use infrastructure::{CorrelationId, TraceContext};
use r#macro::Id;
use tokio::sync::Mutex;

use crate::{
    command::{Command, CommandTarget},
    core::{id::ExternalId, time::DateTime},
    home_state::StateSnapshot,
    notification::NotificationClient,
    observability::system_metric_increment,
    t,
    trigger::UserTriggerId,
};

use super::dispatcher::CommandDispatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
pub enum CommandExecutionResult {
    Debounced,
    AlreadyReflected,
    Executed,
}

#[derive(Debug, Clone)]
struct LastExecution {
    command: Command,
    source: ExternalId,
    created: DateTime,
}

type LastExecutions = HashMap<CommandTarget, LastExecution>;

pub struct CommandService {
    dispatcher: CommandDispatcher,
    notification_client: NotificationClient,
    last_executions: Mutex<LastExecutions>,
}

impl CommandService {
    pub fn new(dispatcher: CommandDispatcher, notification_client: NotificationClient) -> Self {
        Self {
            dispatcher,
            notification_client,
            last_executions: Mutex::new(HashMap::new()),
        }
    }

    #[tracing::instrument(
        name = "execute_command service",
        skip(self, command, snapshot),
        fields(
            command_target = %CommandTarget::from(&command),
            source = %source,
            user_generated = user_trigger_id.is_some(),
            result = tracing::field::Empty,
        )
    )]
    pub async fn execute_command(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        snapshot: &StateSnapshot,
        correlation_id: Option<CorrelationId>,
    ) -> anyhow::Result<CommandExecutionResult> {
        let command_json = serde_json::json!(&command);
        let trace_context = TraceContext::current();
        trace_context.record_json("command", &command_json);

        let mut last_executions = self.last_executions.lock().await;
        let target = CommandTarget::from(&command);
        let outcome = if let Some(last_execution) = last_execution_at(&command, &source, &last_executions)
            && last_execution.elapsed() < t!(30 seconds)
        {
            tracing::trace!(
                "Command for {target} was last executed less than 30 seconds ago, waiting for state update. Skipping for now."
            );
            Ok(CommandExecutionResult::Debounced)
        } else {
            match command.is_reflected_in_state(snapshot, &self.notification_client).await {
                Err(error) => Err(error),
                Ok(true) => {
                    tracing::trace!("Command for {target} is already reflected in state, skipping");
                    Ok(CommandExecutionResult::AlreadyReflected)
                }
                Ok(false) => match self.dispatcher.dispatch(&command).await {
                    Err(error) => Err(error),
                    Ok(()) => {
                        last_executions.insert(
                            target.clone(),
                            LastExecution {
                                command: command.clone(),
                                source: source.clone(),
                                created: t!(now),
                            },
                        );
                        Ok(CommandExecutionResult::Executed)
                    }
                },
            }
        };
        drop(last_executions);

        let result = match &outcome {
            Ok(result) => result.ext_id().variant_name().to_owned(),
            Err(_) => "error".to_owned(),
        };
        let error = outcome.as_ref().err().map(ToString::to_string);
        trace_context.record("result", &result);
        if let Some(error) = &error {
            trace_context.set_error(error.clone());
        } else {
            trace_context.set_ok();
        }

        let metric_target = target.to_string();
        let (command_type, display_target, state) = command.display_parts();
        let command_json =
            serde_json::to_string(&command).unwrap_or_else(|error| format!("<command serialization failed: {error}>"));
        let created = t!(now).to_iso_string();
        let source_name = source.to_string();
        let trace_id = correlation_id.as_ref().map(|id| id.trace_id()).unwrap_or_default();

        system_metric_increment(
            "command_execution",
            &[("target", metric_target.as_str()), ("result", result.as_str())],
        );

        tracing::info!(
            event = "command_executed",
            command_type,
            target = %display_target,
            state = %state,
            command = %command_json,
            created = %created,
            source = %source_name,
            user_generated = user_trigger_id.is_some(),
            execution_result = result.as_str(),
            error = error.as_deref().unwrap_or_default(),
            trace_id = %trace_id,
            "Command executed"
        );

        outcome
    }
}

fn last_execution_at(command: &Command, source: &ExternalId, last_executions: &LastExecutions) -> Option<DateTime> {
    let target: CommandTarget = command.into();
    last_executions
        .get(&target)
        .filter(|execution| execution.source == *source && execution.command == *command)
        .map(|execution| execution.created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PowerToggle;

    #[test]
    fn command_display_parts_match_command_history_format() {
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };

        assert_eq!(
            command.display_parts(),
            ("SetPower", "Dehumidifier".to_string(), "on".to_string())
        );
    }

    #[test]
    fn last_execution_requires_same_source_and_command() {
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        let source = ExternalId::new_static("test", "source");
        let created = t!(now);
        let mut last_executions = LastExecutions::default();
        last_executions.insert(
            command.clone().into(),
            LastExecution {
                command: command.clone(),
                source: source.clone(),
                created,
            },
        );

        assert_eq!(last_execution_at(&command, &source, &last_executions), Some(created));
        assert_eq!(
            last_execution_at(&command, &ExternalId::new_static("test", "other_source"), &last_executions,),
            None
        );
        assert_eq!(
            last_execution_at(
                &Command::SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on: false,
                },
                &source,
                &last_executions,
            ),
            None
        );
    }

    #[test]
    fn execution_result_ids_are_stable() {
        assert_eq!(
            CommandExecutionResult::Debounced.ext_id().type_name(),
            "command_execution_result"
        );
        assert_eq!(
            CommandExecutionResult::AlreadyReflected.ext_id().variant_name(),
            "already_reflected"
        );
        assert_eq!(CommandExecutionResult::Executed.ext_id().variant_name(), "executed");
    }
}
