use infrastructure::TraceContext;
use r#macro::Id;

use crate::{
    command::{
        Command, CommandTarget,
        adapter::{CommandExecution, CommandExecutionRepository},
    },
    core::id::ExternalId,
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

pub struct CommandService {
    dispatcher: CommandDispatcher,
    notification_client: NotificationClient,
    execution_repository: CommandExecutionRepository,
}

impl CommandService {
    pub fn new(
        dispatcher: CommandDispatcher,
        notification_client: NotificationClient,
        execution_repository: CommandExecutionRepository,
    ) -> Self {
        Self {
            dispatcher,
            notification_client,
            execution_repository,
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
        let target = CommandTarget::from(&command);
        let latest_execution = self.execution_repository.latest_for(&target).await;
        let source_changed = source_changed(latest_execution.as_ref(), &source);

        let outcome = if source_changed {
            self.dispatch_and_record(&command, &source).await
        } else if let Some(last_execution) = latest_execution.as_ref()
            && last_execution.source == source
            && last_execution.command == command
            && last_execution.created.elapsed() < t!(30 seconds)
        {
            Ok(CommandExecutionResult::Debounced)
        } else {
            match command.is_reflected_in_state(snapshot, &self.notification_client).await {
                Err(error) => Err(error),
                Ok(true) => Ok(CommandExecutionResult::AlreadyReflected),
                Ok(false) => self.dispatch_and_record(&command, &source).await,
            }
        };

        add_to_trace(&command, &source, user_trigger_id.is_some(), &outcome);

        outcome
    }

    async fn dispatch_and_record(
        &self,
        command: &Command,
        source: &ExternalId,
    ) -> anyhow::Result<CommandExecutionResult> {
        self.dispatcher.dispatch(command).await?;
        self.execution_repository
            .record_execution(command.clone(), source.clone())
            .await;
        Ok(CommandExecutionResult::Executed)
    }
}

fn source_changed(last_execution: Option<&CommandExecution>, source: &ExternalId) -> bool {
    last_execution.is_some_and(|execution| execution.source != *source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PowerToggle;

    #[test]
    fn source_change_is_detected_within_the_debounce_window() {
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        let previous_source = ExternalId::new_static("test", "previous");
        let current_source = ExternalId::new_static("test", "current");
        let last_execution = CommandExecution {
            command,
            source: previous_source.clone(),
            created: crate::core::time::DateTime::now(),
        };

        assert!(source_changed(Some(&last_execution), &current_source));
        assert!(!source_changed(Some(&last_execution), &previous_source));
        assert!(!source_changed(None, &current_source));
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
