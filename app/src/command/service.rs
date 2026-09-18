use infrastructure::CorrelationId;

use crate::{
    command::{Command, CommandExecution, CommandState, CommandTarget},
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange},
    },
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
        let commands = self.repo.query_all_commands(Some(target), &range).await?;
        Ok(self
            .apply_timeshift_filter(commands, |cmd| cmd.created)
            .into_iter()
            .max_by_key(|cmd| cmd.created))
    }

    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> anyhow::Result<Vec<CommandExecution>> {
        let commands = self
            .repo
            .query_all_commands(None, &DateTimeRange::new(from, until))
            .await?;
        Ok(self.apply_timeshift_filter(commands, |cmd| cmd.created))
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
