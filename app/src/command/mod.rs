mod adapter;
mod domain;
mod service;

pub use domain::*;

use std::sync::Arc;

use adapter::db::CommandRepository;
use infrastructure::{EventBus, EventListener, Mqtt, TraceContext};
use service::CommandService;
use sqlx::PgPool;

use crate::{
    core::{id::ExternalId, time::DateTime},
    trigger::UserTriggerId,
};

#[derive(Debug, Clone)]
pub enum CommandEvent {
    CommandExecuted(CommandExecution),
}

pub struct CommandModule {
    service: Arc<CommandService>,
    event_bus: EventBus<CommandEvent>,
    z2m_sender_runner: adapter::z2m::sender::Z2mSenderRunner,
}

#[derive(Clone)]
pub struct CommandClient {
    service: Arc<CommandService>,
}

impl CommandModule {
    pub async fn new(
        pool: PgPool,
        mqtt_client: &mut Mqtt,
        tasmota_event_topic: &str,
        z2m_event_topic: &str,
        ha_url: &str,
        ha_token: &str,
    ) -> Self {
        let repo = CommandRepository::new(pool);
        let event_bus = EventBus::new(64);

        let mqtt_sender = mqtt_client.sender();
        let tasmota_executor = adapter::TasmotaCommandExecutor::new(tasmota_event_topic, mqtt_sender.clone());

        let (z2m_sender, z2m_sender_runner) = adapter::z2m::sender::Z2mSender::new(mqtt_client, z2m_event_topic)
            .await
            .expect("Failed to initialize Z2M sender");
        let z2m_executor = adapter::Z2mCommandExecutor::new(z2m_sender);
        let ha_executor = adapter::HomeAssistantCommandExecutor::new(ha_url, ha_token);

        let service = Arc::new(CommandService::new(
            repo,
            tasmota_executor,
            z2m_executor,
            ha_executor,
            event_bus.emitter(),
        ));

        Self {
            service,
            event_bus,
            z2m_sender_runner,
        }
    }

    pub fn client(&self) -> CommandClient {
        CommandClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> EventListener<CommandEvent> {
        self.event_bus.subscribe()
    }

    pub async fn run(self) {
        self.z2m_sender_runner.run().await;
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
