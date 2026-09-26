use moka::future::Cache;

use crate::{
    command::{Command, CommandTarget},
    core::time::DateTime,
    t,
};

#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub command: Command,
    pub created: DateTime,
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
    pub async fn latest_for(&self, target: &CommandTarget) -> Option<CommandExecution> {
        self.last_executions.get(target).await
    }

    pub async fn record_execution(&self, command: Command) {
        let target = CommandTarget::from(&command);
        self.last_executions
            .insert(
                target,
                CommandExecution {
                    command,
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
    async fn latest_execution_is_loaded_by_target_with_its_command() {
        let repository = CommandExecutionRepository::default();
        let command = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };
        repository.record_execution(command.clone()).await;

        let other_state = Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: false,
        };
        let latest = repository
            .latest_for(&CommandTarget::from(&other_state))
            .await
            .expect("execution was recorded");

        assert_eq!(latest.command, command);
    }
}
