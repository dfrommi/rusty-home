use tokio::sync::broadcast;

use crate::{
    command::{Command, CommandEvent, CommandExecution, CommandState, CommandTarget},
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange},
    },
    t,
    trigger::UserTriggerId,
};

use super::adapter::db::CommandRepository;

pub struct CommandService {
    repo: CommandRepository,
    event_tx: broadcast::Sender<CommandEvent>,
}

impl CommandService {
    pub fn new(repo: CommandRepository, event_tx: broadcast::Sender<CommandEvent>) -> Self {
        Self { repo, event_tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CommandEvent> {
        self.event_tx.subscribe()
    }

    pub async fn save_command(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        correlation_id: Option<String>,
    ) -> anyhow::Result<()> {
        let execution = self
            .repo
            .insert_command(&command, &source, user_trigger_id, correlation_id)
            .await?;

        let _ = self.event_tx.send(CommandEvent::Added(execution));

        Ok(())
    }

    pub async fn get_command_for_processing(&self) -> anyhow::Result<Option<CommandExecution>> {
        let result = self.repo.get_command_for_processing().await?;

        if let Some(cmd) = result.as_ref() {
            let _ = self.event_tx.send(CommandEvent::Started(cmd.clone()));
        }

        Ok(result)
    }

    pub async fn set_command_state_success(&self, command_id: i64) -> anyhow::Result<()> {
        self.repo.set_command_state_success(command_id).await?;

        let _ = self.event_tx.send(CommandEvent::Finished {
            id: command_id,
            state: CommandState::Success,
        });

        Ok(())
    }

    pub async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> anyhow::Result<()> {
        self.repo.set_command_state_error(command_id, error_message).await?;

        let _ = self.event_tx.send(CommandEvent::Finished {
            id: command_id,
            state: CommandState::Error(error_message.to_string()),
        });

        Ok(())
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

    fn apply_timeshift_filter<T>(&self, items: Vec<T>, get_timestamp: impl Fn(&T) -> DateTime) -> Vec<T> {
        if DateTime::is_shifted() {
            let now = t!(now);
            items.into_iter().filter(|item| get_timestamp(item) <= now).collect()
        } else {
            items
        }
    }
}
