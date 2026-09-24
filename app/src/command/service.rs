use std::collections::HashMap;

use infrastructure::TraceContext;
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
        name = "execute_command",
        skip_all,
        fields(
            source = %source,
            user_generated = user_trigger_id.is_some(),
            command_type = command.display_parts().command_type,
            target = command.display_parts().target,
            state = command.display_parts().state,
            outcome = tracing::field::Empty,
        )
    )]
    pub async fn execute_command(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>, //TODO correlation_id instead of DB id
        snapshot: &StateSnapshot,
    ) -> anyhow::Result<CommandExecutionResult> {
        let mut last_executions = self.last_executions.lock().await;

        let outcome = if let Some(last_execution) = last_execution_at(&command, &source, &last_executions)
            && last_execution.elapsed() < t!(30 seconds)
        {
            Ok(CommandExecutionResult::Debounced)
        } else {
            match command.is_reflected_in_state(snapshot, &self.notification_client).await {
                Err(error) => Err(error),
                Ok(true) => Ok(CommandExecutionResult::AlreadyReflected),
                Ok(false) => match self.dispatcher.dispatch(&command).await {
                    Err(error) => Err(error),
                    Ok(()) => {
                        last_executions.insert(
                            CommandTarget::from(&command),
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

        add_to_trace(&command, &source, user_trigger_id.is_some(), &outcome);

        outcome
    }
}

fn add_to_trace(
    command: &Command,
    source: &ExternalId,
    user_generated: bool,
    outcome: &anyhow::Result<CommandExecutionResult>,
) {
    let target = CommandTarget::from(command);
    let display_parts = command.display_parts();

    let current_trace = TraceContext::current();

    let result = match &outcome {
        Ok(result) => result.ext_id().variant_name().to_owned(),
        Err(_) => "error".to_owned(),
    };

    current_trace
        .record("command_type", display_parts.command_type)
        .record("command_target", &display_parts.target)
        .record("state", &display_parts.state)
        .record("source", source.to_string())
        .record("user_generated", user_generated.to_string())
        .record("outcome", result.as_str())
        .record_json("command", &serde_json::json!(command));

    match &outcome {
        Ok(_) => current_trace.set_ok(),
        Err(error) => current_trace.set_error(error.to_string()),
    }

    let metric_target = target.to_string();
    system_metric_increment(
        "command_execution",
        &[("target", metric_target.as_str()), ("result", result.as_str())],
    );

    match &outcome {
        Ok(CommandExecutionResult::Executed) => tracing::info!(
            event = "command_executed",
            command_type = display_parts.command_type,
            command_target = display_parts.target,
            state = display_parts.state,
            user_generated = user_generated,
            source = source.to_string(),
            "Command executed"
        ),
        Err(e) => tracing::error!(
            event = "command_error",
            command_type = display_parts.command_type,
            command_target = display_parts.target,
            state = display_parts.state,
            user_generated = user_generated,
            source = source.to_string(),
            "Command execution failed: {}",
            e
        ),
        Ok(CommandExecutionResult::Debounced | CommandExecutionResult::AlreadyReflected) => {}
    }
}

fn last_execution_at(command: &Command, source: &ExternalId, last_executions: &LastExecutions) -> Option<DateTime> {
    let target: CommandTarget = command.into();
    last_executions
        .get(&target)
        .filter(|execution| execution.source == *source && execution.command == *command)
        .map(|execution| execution.created)
}
