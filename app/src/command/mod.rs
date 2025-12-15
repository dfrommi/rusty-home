mod adapter;
mod domain;
mod service;

pub use domain::*;

use std::sync::Arc;

use adapter::db::CommandRepository;
use infrastructure::{MqttOutMessage, TraceContext};
use service::CommandService;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};

use crate::{
    core::{id::ExternalId, time::DateTime},
    trigger::UserTriggerId,
};

#[derive(Debug, Clone)]
pub enum CommandEvent {
    Added(CommandExecution),
    Started(CommandExecution),
    Finished { id: i64, state: CommandState },
}

pub struct CommandRunner {
    service: Arc<CommandService>,
    event_rx: broadcast::Receiver<CommandEvent>,
    pending_tx: broadcast::Sender<CommandExecution>,
}

#[derive(Clone)]
pub struct CommandClient {
    service: Arc<CommandService>,
}

impl CommandRunner {
    pub fn new(
        pool: PgPool,
        mqtt_sender: mpsc::Sender<MqttOutMessage>,
        tasmota_event_topic: &str,
        z2m_event_topic: &str,
    ) -> Self {
        let repo = CommandRepository::new(pool);
        let (event_tx, _event_rx) = broadcast::channel(64);

        let tasmota_executor = adapter::TasmotaCommandExecutor::new(tasmota_event_topic, mqtt_sender.clone());
        let z2m_executor = adapter::Z2mCommandExecutor::new(mqtt_sender, z2m_event_topic);

        let service = Arc::new(CommandService::new(repo, tasmota_executor, z2m_executor, event_tx));

        let event_rx = service.subscribe();
        let pending_tx = broadcast::channel(32).0;

        Self {
            service,
            event_rx,
            pending_tx,
        }
    }

    pub fn client(&self) -> CommandClient {
        CommandClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CommandEvent> {
        self.service.subscribe()
    }

    pub fn subscribe_pending_commands(&self) -> broadcast::Receiver<CommandExecution> {
        self.pending_tx.subscribe()
    }

    pub async fn run_dispatcher(mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(15));
        let mut got_cmd = false;

        loop {
            if !got_cmd {
                tokio::select! {
                    _ = self.event_rx.recv() => {},
                    _ = timer.tick() => {},
                };
            }

            match self.service.get_command_for_processing().await {
                Ok(Some(cmd)) => {
                    got_cmd = true;
                    self.pending_tx.send(cmd).ok();
                }
                Ok(None) => got_cmd = false,
                Err(e) => {
                    tracing::error!("Error getting pending commands: {:?}", e);
                    got_cmd = false;
                }
            }
        }
    }
}

impl CommandClient {
    pub fn subscribe_events(&self) -> broadcast::Receiver<CommandEvent> {
        self.service.subscribe()
    }

    pub async fn execute(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
    ) -> anyhow::Result<()> {
        self.service
            .execute_command(command, source, user_trigger_id, TraceContext::current_correlation_id())
            .await
    }

    pub async fn enqueue(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
    ) -> anyhow::Result<()> {
        self.service
            .save_command(command, source, user_trigger_id, TraceContext::current_correlation_id())
            .await
    }

    pub async fn get_command_for_processing(&self) -> anyhow::Result<Option<CommandExecution>> {
        self.service.get_command_for_processing().await
    }

    pub async fn set_command_state_success(&self, command_id: i64) -> anyhow::Result<()> {
        self.service.set_command_state_success(command_id).await
    }

    pub async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> anyhow::Result<()> {
        self.service.set_command_state_error(command_id, error_message).await
    }

    pub async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> anyhow::Result<Option<CommandExecution>> {
        self.service.get_latest_command(target.into(), since).await
    }

    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> anyhow::Result<Vec<CommandExecution>> {
        self.service.get_all_commands(from, until).await
    }
}
