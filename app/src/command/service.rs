use infrastructure::EventEmitter;

use crate::{
    command::{
        Command, CommandEvent, CommandExecution, CommandState, CommandTarget,
        adapter::{CommandExecutor, HomeAssistantCommandExecutor, TasmotaCommandExecutor, Z2mCommandExecutor},
    },
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
    tasmota_executor: TasmotaCommandExecutor,
    z2m_executor: Z2mCommandExecutor,
    ha_executor: HomeAssistantCommandExecutor,
    event_tx: EventEmitter<CommandEvent>,
}

impl CommandService {
    pub fn new(
        repo: CommandRepository,
        tasmota_executor: TasmotaCommandExecutor,
        z2m_executor: Z2mCommandExecutor,
        ha_executor: HomeAssistantCommandExecutor,
        event_tx: EventEmitter<CommandEvent>,
    ) -> Self {
        Self {
            repo,
            tasmota_executor,
            z2m_executor,
            ha_executor,
            event_tx,
        }
    }

    pub async fn execute_command(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        correlation_id: Option<String>,
    ) -> anyhow::Result<CommandExecution> {
        let mut command_exec = self
            .repo
            .insert_command_for_processing(&command, &source, user_trigger_id, correlation_id)
            .await?;

        let command_id = command_exec.id;

        let res = match self.execute_via(&self.tasmota_executor, &command).await {
            Some(r) => Some(r),
            None => match self.execute_via(&self.z2m_executor, &command).await {
                Some(r) => Some(r),
                None => self.execute_via(&self.ha_executor, &command).await,
            },
        };

        let final_state = match res {
            Some(Ok(())) => CommandState::Success,
            Some(Err(e)) => CommandState::Error(e.to_string()),
            None => CommandState::Error("No executor".to_string()),
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

        self.event_tx.send(CommandEvent::CommandExecuted(command_exec.clone()));

        Ok(command_exec)
    }

    async fn execute_via(&self, executor: &impl CommandExecutor, command: &Command) -> Option<anyhow::Result<()>> {
        match executor.execute_command(command).await {
            Ok(true) => Some(Ok(())),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
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
