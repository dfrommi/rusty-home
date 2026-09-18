use infrastructure::{CorrelationId, TraceContext};

use crate::{
    command::{Command, CommandTarget},
    core::id::ExternalId,
    observability::system_metric_increment,
    t,
    trigger::UserTriggerId,
};

use super::dispatcher::CommandDispatcher;

pub struct CommandService {
    dispatcher: CommandDispatcher,
}

impl CommandService {
    pub fn new(dispatcher: CommandDispatcher) -> Self {
        Self { dispatcher }
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
    ) -> anyhow::Result<()> {
        let command_json = serde_json::json!(&command);
        let trace_context = TraceContext::current();
        trace_context.record_json("command", &command_json);

        let (result, error) = match self.dispatcher.dispatch(&command).await {
            Ok(()) => ("success", None),
            Err(error) => ("error", Some(error.to_string())),
        };
        trace_context.record("result", result);
        if let Some(error) = &error {
            trace_context.set_error(error.clone());
        } else {
            trace_context.set_ok();
        }

        let target = CommandTarget::from(&command);
        let metric_target = target.to_string();
        let (command_type, display_target, state) = command.display_parts();
        let command_json =
            serde_json::to_string(&command).unwrap_or_else(|error| format!("<command serialization failed: {error}>"));
        let created = t!(now).to_iso_string();
        let source_name = source.to_string();
        let trace_id = correlation_id.as_ref().map(|id| id.trace_id()).unwrap_or_default();

        system_metric_increment("command_execution", &[("target", metric_target.as_str()), ("result", result)]);

        tracing::info!(
            event = "command_executed",
            command_type,
            target = %display_target,
            state = %state,
            command = %command_json,
            created = %created,
            source = %source_name,
            user_generated = user_trigger_id.is_some(),
            execution_result = result,
            error = error.as_deref().unwrap_or_default(),
            trace_id = %trace_id,
            "Command executed"
        );

        Ok(())
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
