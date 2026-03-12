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
    home_state::HomeStateEvent,
    trigger::UserTriggerId,
};

#[derive(Debug, Clone)]
pub enum CommandEvent {
    CommandExecuted(CommandExecution),
}

pub struct CommandModule {
    service: Arc<CommandService>,
    z2m_sensor_sync_runner: adapter::z2m::Z2mSensorSyncRunner,
}

#[derive(Clone)]
pub struct CommandClient {
    service: Arc<CommandService>,
}

impl CommandModule {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        event_bus: EventBus<CommandEvent>,
        pool: PgPool,
        mqtt_client: &mut Mqtt,
        tasmota_event_topic: &str,
        z2m_event_topic: &str,
        ha_url: &str,
        ha_token: &str,
        home_state_listener: EventListener<HomeStateEvent>,
    ) -> Self {
        let repo = CommandRepository::new(pool);

        let tasmota_executor = adapter::TasmotaCommandExecutor::new(mqtt_client.sender(tasmota_event_topic));
        let ha_executor = adapter::HomeAssistantCommandExecutor::new(ha_url, ha_token);
        let z2m_executor = adapter::Z2mCommandExecutor::new(mqtt_client.sender(z2m_event_topic));

        let z2m_sensor_sync_runner =
            adapter::z2m::Z2mSensorSyncRunner::new(mqtt_client.sender(z2m_event_topic), home_state_listener);

        let service = Arc::new(CommandService::new(
            repo,
            tasmota_executor,
            z2m_executor,
            ha_executor,
            event_bus.emitter(),
        ));

        Self {
            service,
            z2m_sensor_sync_runner,
        }
    }

    pub fn client(&self) -> CommandClient {
        CommandClient {
            service: self.service.clone(),
        }
    }

    pub async fn run(self) {
        self.z2m_sensor_sync_runner.run().await;
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
            .execute_command(command, source, user_trigger_id, TraceContext::current().correlation_id())
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
