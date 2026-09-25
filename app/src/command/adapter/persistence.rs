use moka::future::Cache;

use crate::{
    command::{Command, CommandTarget},
    core::{id::ExternalId, time::DateTime},
    t,
};

#[derive(Debug, Clone)]
struct CommandExecution {
    command: Command,
    source: ExternalId,
    created: DateTime,
}

pub struct CommandExecutionRepository {
    last_executions: Cache<CommandTarget, CommandExecution>,
}

impl Default for CommandExecutionRepository {
    fn default() -> Self {
        Self {
            last_executions: Cache::builder().build(),
        }
    }
}

impl CommandExecutionRepository {
    pub async fn last_execution_at(&self, command: &Command, source: &ExternalId) -> Option<DateTime> {
        let target = CommandTarget::from(command);
        self.last_executions
            .get(&target)
            .await
            .filter(|execution| execution.source == *source && execution.command == *command)
            .map(|execution| execution.created)
    }

    pub async fn record_execution(&self, command: Command, source: ExternalId) {
        let target = CommandTarget::from(&command);
        self.last_executions
            .insert(
                target,
                CommandExecution {
                    command,
                    source,
                    created: t!(now),
                },
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PowerToggle;

    #[tokio::test]
    async fn latest_execution_matches_the_same_command_and_source() {
        let repository = CommandExecutionRepository::default();
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        let source = ExternalId::new_static("test", "source");
        let other_source = ExternalId::new_static("test", "other_source");

        repository.record_execution(command.clone(), source.clone()).await;

        assert!(repository.last_execution_at(&command, &source).await.is_some());
        assert!(repository.last_execution_at(&command, &other_source).await.is_none());
        assert!(
            repository
                .last_execution_at(
                    &Command::SetPower {
                        device: PowerToggle::Dehumidifier,
                        power_on: false,
                    },
                    &source,
                )
                .await
                .is_none()
        );
    }
}
