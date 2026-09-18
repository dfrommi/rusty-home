use infrastructure::{CorrelationId, TraceContext};

use crate::{
    command::{Command, CommandExecution, CommandState, CommandTarget},
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange},
    },
    observability::system_metric_increment,
    t,
    trigger::UserTriggerId,
};

use super::{adapter::db::CommandRepository, dispatcher::CommandDispatcher};

pub struct CommandService {
    repo: CommandRepository,
    dispatcher: CommandDispatcher,
}

impl CommandService {
    pub fn new(repo: CommandRepository, dispatcher: CommandDispatcher) -> Self {
        Self { repo, dispatcher }
    }

    #[tracing::instrument(
        name = "execute_command service",
        skip(self, command),
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
        correlation_id: Option<CorrelationId>,
    ) -> anyhow::Result<CommandExecution> {
        let mut command_exec = self
            .repo
            .insert_command_for_processing(&command, &source, user_trigger_id, correlation_id)
            .await?;

        let command_id = command_exec.id;
        let final_state = match self.dispatcher.dispatch(&command).await {
            Ok(()) => CommandState::Success,
            Err(e) => CommandState::Error(e.to_string()),
        };

        command_exec.state = final_state.clone();

        let trace_context = TraceContext::current();
        let command_json = serde_json::json!(&command_exec.command);
        trace_context.record_json("command", &command_json);

        let (result, error) = match &final_state {
            CommandState::Success => ("success", None),
            CommandState::Error(error) => ("error", Some(error.as_str())),
            CommandState::Pending | CommandState::InProgress => ("in_progress", None),
        };
        trace_context.record("result", result);
        if let Some(error) = error {
            trace_context.set_error(error.to_owned());
        } else {
            trace_context.set_ok();
        }

        let target = CommandTarget::from(&command_exec.command);
        let metric_target = target.to_string();
        let (command_type, display_target, state) = command_exec.command.display_parts();
        let command_json = serde_json::to_string(&command_exec.command)
            .unwrap_or_else(|error| format!("<command serialization failed: {error}>"));
        let created = command_exec.created.to_iso_string();
        let source = command_exec.source.to_string();
        let trace_id = command_exec
            .correlation_id
            .as_ref()
            .map(|id| id.trace_id())
            .unwrap_or_default();

        system_metric_increment("command_execution", &[("target", metric_target.as_str()), ("result", result)]);

        tracing::info!(
            event = "command_executed",
            command_type,
            target = %display_target,
            state = %state,
            command = %command_json,
            created = %created,
            source = %source,
            user_generated = command_exec.is_user_generated(),
            execution_result = result,
            error = error.unwrap_or_default(),
            trace_id = %trace_id,
            "Command executed"
        );

        if let Err(e) = self.repo.set_command_state(command_id, final_state.clone()).await {
            tracing::warn!(
                "Failed to update command state of {} to {:?} in DB: {}",
                command_id,
                final_state,
                e
            );
        }

        Ok(command_exec)
    }

    pub async fn get_latest_command(
        &self,
        target: CommandTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<CommandExecution>> {
        let range = DateTimeRange::new(since, t!(now));
        let commands = self.repo.query_commands_for_target(target, &range).await?;
        Ok(self
            .apply_timeshift_filter(commands, |cmd| cmd.created)
            .into_iter()
            .max_by_key(|cmd| cmd.created))
    }

    //TODO why not on DB?
    fn apply_timeshift_filter<T>(&self, items: Vec<T>, get_timestamp: impl Fn(&T) -> DateTime) -> Vec<T> {
        if DateTime::is_shifted() {
            let now = t!(now);
            items.into_iter().filter(|item| get_timestamp(item) <= now).collect()
        } else {
            items
        }
    }
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
}
