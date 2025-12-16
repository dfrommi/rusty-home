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
    device_state::DeviceStateClient,
    trigger::UserTriggerId,
};

#[derive(Debug, Clone)]
pub enum CommandEvent {
    CommandExecuted(CommandExecution),
}

pub struct CommandRunner {
    service: Arc<CommandService>,
    event_rx: broadcast::Receiver<CommandEvent>,
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
        ha_url: &str,
        ha_token: &str,
        //TODO handle via command-events
        device_client: DeviceStateClient,
    ) -> Self {
        let repo = CommandRepository::new(pool);
        let (event_tx, _event_rx) = broadcast::channel(64);

        let tasmota_executor = adapter::TasmotaCommandExecutor::new(tasmota_event_topic, mqtt_sender.clone());
        let z2m_executor = adapter::Z2mCommandExecutor::new(mqtt_sender, z2m_event_topic);
        let ha_executor = adapter::HomeAssistantCommandExecutor::new(ha_url, ha_token, device_client);

        let service = Arc::new(CommandService::new(repo, tasmota_executor, z2m_executor, ha_executor, event_tx));

        let event_rx = service.subscribe();

        Self { service, event_rx }
    }

    pub fn client(&self) -> CommandClient {
        CommandClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CommandEvent> {
        self.service.subscribe()
    }
}

impl CommandClient {
    pub async fn execute(
        &self,
        command: Command,
        source: ExternalId,
        user_trigger_id: Option<UserTriggerId>,
    ) -> anyhow::Result<CommandExecution> {
        self.service
            .execute_command(command, source, user_trigger_id, TraceContext::current_correlation_id())
            .await
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
