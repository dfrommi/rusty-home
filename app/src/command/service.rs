use infrastructure::TraceContext;
use r#macro::Id;

use crate::{
    command::{
        Command, CommandTarget,
        adapter::{CommandExecution, CommandExecutionRepository},
    },
    core::id::ExternalId,
    home_state::StateSnapshot,
    observability::system_metric_increment,
    t,
    trigger::UserTriggerId,
};

use super::dispatcher::CommandDispatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
pub enum CommandExecutionResult {
    Debounced,
    AlreadyReflected,
    AlreadyExecuted,
    Executed,
}

pub struct CommandService {
    dispatcher: CommandDispatcher,
    execution_repository: CommandExecutionRepository,
}

impl CommandService {
    pub fn new(dispatcher: CommandDispatcher, execution_repository: CommandExecutionRepository) -> Self {
        Self {
            dispatcher,
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
        let reflection = command.is_reflected_in_state(snapshot)?;
        let outcome = match execution_decision(&command, reflection, latest_execution.as_ref()) {
            ExecutionDecision::Debounced => Ok(CommandExecutionResult::Debounced),
            ExecutionDecision::AlreadyReflected => Ok(CommandExecutionResult::AlreadyReflected),
            ExecutionDecision::AlreadyExecuted => Ok(CommandExecutionResult::AlreadyExecuted),
            ExecutionDecision::Execute => self.dispatch_and_record(&command).await,
        };

        add_to_trace(&command, &source, user_trigger_id.is_some(), &outcome);

        outcome
    }

    async fn dispatch_and_record(&self, command: &Command) -> anyhow::Result<CommandExecutionResult> {
        self.dispatcher.dispatch(command).await?;
        self.execution_repository.record_execution(command.clone()).await;
        Ok(CommandExecutionResult::Executed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionDecision {
    Debounced,
    AlreadyReflected,
    AlreadyExecuted,
    Execute,
}

fn execution_decision(
    command: &Command,
    reflection: Option<bool>,
    latest_execution: Option<&CommandExecution>,
) -> ExecutionDecision {
    let same_command = latest_execution.is_some_and(|execution| execution.command == *command);
    let recent_same_command = latest_execution
        .is_some_and(|execution| execution.command == *command && execution.created.elapsed() < t!(30 seconds));

    match reflection {
        Some(true) => ExecutionDecision::AlreadyReflected,
        Some(false) if recent_same_command => ExecutionDecision::Debounced,
        Some(false) => ExecutionDecision::Execute,
        None if command.deduplicate_when_unobservable() && same_command => ExecutionDecision::AlreadyExecuted,
        None if recent_same_command => ExecutionDecision::Debounced,
        None => ExecutionDecision::Execute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{
        NotificationDestination, NotificationKind, NotificationOperation, NotificationRecipient, PowerToggle,
    };

    fn execution(command: Command, elapsed: crate::core::time::Duration) -> CommandExecution {
        CommandExecution {
            command,
            created: crate::core::time::DateTime::now() - elapsed,
        }
    }

    fn phone_notification(operation: NotificationOperation) -> Command {
        Command::Notify {
            notification: NotificationKind::WindowOpened,
            target: NotificationDestination::Phone {
                recipient: NotificationRecipient::Dennis,
            },
            operation,
        }
    }

    #[test]
    fn reflected_state_takes_priority_over_recent_execution() {
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        let recent = execution(command.clone(), crate::t!(1 seconds));

        assert_eq!(
            execution_decision(&command, Some(true), Some(&recent)),
            ExecutionDecision::AlreadyReflected
        );
    }

    #[test]
    fn unreflected_state_retries_after_the_debounce_window() {
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        let recent = execution(command.clone(), crate::t!(1 seconds));
        let stale = execution(command.clone(), crate::t!(1 minutes));

        assert_eq!(
            execution_decision(&command, Some(false), Some(&recent)),
            ExecutionDecision::Debounced
        );
        assert_eq!(
            execution_decision(&command, Some(false), Some(&stale)),
            ExecutionDecision::Execute
        );
    }

    #[test]
    fn unobservable_notification_deduplicates_by_successful_command_equality() {
        let command = phone_notification(NotificationOperation::Show);
        let previous = execution(command.clone(), crate::t!(1 minutes));
        let dismiss = execution(phone_notification(NotificationOperation::Dismiss), crate::t!(1 minutes));

        assert_eq!(
            execution_decision(&command, None, Some(&previous)),
            ExecutionDecision::AlreadyExecuted
        );
        assert_eq!(execution_decision(&command, None, Some(&dismiss)), ExecutionDecision::Execute);
    }

    #[test]
    fn one_shot_commands_remain_retryable_after_debounce() {
        let command = Command::OpenDoor {
            device: crate::command::Lock::BuildingEntrance,
        };
        let recent = execution(command.clone(), crate::t!(1 seconds));
        let stale = execution(command.clone(), crate::t!(1 minutes));

        assert_eq!(execution_decision(&command, None, Some(&recent)), ExecutionDecision::Debounced);
        assert_eq!(execution_decision(&command, None, Some(&stale)), ExecutionDecision::Execute);
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
        Ok(
            CommandExecutionResult::Debounced
            | CommandExecutionResult::AlreadyReflected
            | CommandExecutionResult::AlreadyExecuted,
        ) => {}
    }
}
