use anyhow::Context;
use api::{DbEventListener, command::Command};
use config::{
    default_ha_command_config, default_ha_state_config, default_tasmota_command_config,
    default_tasmota_state_config, default_z2m_state_config,
};
use core::{
    CommandExecutor, IncomingData, IncomingDataProcessor, IncomingMqttDataProcessor,
    event::{AppEventListener, CommandAddedEvent},
    process_incoming_data_source,
};
use homeassistant::HaCommandExecutor;
use infrastructure::MqttInMessage;
use settings::Settings;
use tasmota::{TasmotaCommandExecutor, TasmotaIncomingDataSource};
use tokio::sync::{broadcast::Receiver, mpsc};
use z2m::Z2mIncomingDataSource;

use sqlx::PgPool;

mod config;
mod core;
mod energy_meter;
mod homeassistant;
mod settings;
mod tasmota;
mod z2m;

type IncomingDataSender = mpsc::Sender<IncomingData>;

struct Infrastructure {
    database: Database,
    event_listener: AppEventListener,
    mqtt_client: infrastructure::Mqtt,
    incoming_data_tx: IncomingDataSender,
}

#[derive(Clone)]
pub struct Database {
    db_pool: PgPool,
}

impl Database {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    settings
        .monitoring
        .init()
        .expect("Error initializing monitoring");

    let (mut infrastructure, incoming_data_rx) = Infrastructure::init(&settings).await.unwrap();

    let energy_meter_processing = {
        let mut energy_meter_processor = energy_meter::new(
            infrastructure.database.clone(),
            infrastructure
                .event_listener
                .new_energy_reading_added_listener(),
        );
        let tx = infrastructure.incoming_data_tx.clone();

        async move {
            energy_meter_processor
                .process(tx)
                .await
                .expect("Error processing energy meter incoming data");
        }
    };

    let ha_incoming_data_processing = {
        let mut ha_incoming_data_processor = settings
            .homeassistant
            .new_incoming_data_processor(&mut infrastructure)
            .await;
        let tx = infrastructure.incoming_data_tx.clone();

        async move {
            ha_incoming_data_processor
                .process(tx.clone())
                .await
                .expect("Error processing HA incoming data");
        }
    };

    let z2m_incoming_data_processing = {
        let z2m_incoming_data_source = settings
            .z2m
            .new_incoming_data_source(&mut infrastructure)
            .await;
        let db = infrastructure.database.clone();
        async move {
            process_incoming_data_source("Z2M", z2m_incoming_data_source, &db)
                .await
                .expect("Error processing Z2M incoming data");
        }
    };

    let tasmota_incoming_data_processing = {
        let tasmota_incoming_data_source = settings
            .tasmota
            .new_incoming_data_source(&mut infrastructure)
            .await;
        let db = infrastructure.database.clone();
        async move {
            process_incoming_data_source("Tasmota", tasmota_incoming_data_source, &db)
                .await
                .expect("Error processing Tasmota incoming data");
        }
    };

    let incoming_data_persisting = {
        let storage = infrastructure.database.clone();

        async move {
            core::collect_states(incoming_data_rx, &storage)
                .await
                .expect("Error persisting incoming data");
        }
    };

    let execute_commands = {
        let command_repo = infrastructure.database.clone();
        let new_cmd_available = infrastructure.new_command_available_listener();
        let ha_cmd_executor = settings
            .homeassistant
            .new_command_executor(infrastructure.incoming_data_tx.clone());
        let tasmota_cmd_executor = settings.tasmota.new_command_executor(&infrastructure);

        let cmd_executor = MultiCommandExecutor {
            primary: ha_cmd_executor,
            secondary: tasmota_cmd_executor,
        };

        async move {
            core::execute_commands(&command_repo, &cmd_executor, new_cmd_available).await;
        }
    };

    let http_server_exec = {
        let http_db = infrastructure.database.clone();

        async move {
            settings
                .http_server
                .run_server(move || vec![energy_meter::new_web_service(http_db.clone())])
                .await
                .expect("HTTP server execution failed");
        }
    };

    let process_infrastucture = infrastructure.process();

    tokio::select!(
        _ = energy_meter_processing => {},
        _ = ha_incoming_data_processing => {},
        _ = z2m_incoming_data_processing => {},
        _ = tasmota_incoming_data_processing => {},
        _ = incoming_data_persisting => {},
        _ = execute_commands => {},
        _ = http_server_exec => {},
        _ = process_infrastucture => {},
    );
}

impl settings::HomeAssitant {
    fn new_command_executor(&self, incoming_data_tx: IncomingDataSender) -> impl CommandExecutor {
        let http_client = homeassistant::HaHttpClient::new(&self.url, &self.token)
            .expect("Error initializing Home Assistant REST client");
        HaCommandExecutor::new(http_client, incoming_data_tx, &default_ha_command_config())
    }

    async fn new_incoming_data_processor(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl IncomingDataProcessor + use<> {
        let http_client = homeassistant::HaHttpClient::new(&self.url, &self.token)
            .expect("Error initializing Home Assistant REST client");

        let mqtt_client = homeassistant::HaMqttClient::new(
            infrastructure
                .subscribe_to_mqtt(&self.topic_event)
                .await
                .expect("Error subscribing to MQTT topic"),
        );

        homeassistant::new_incoming_data_processor(
            http_client,
            mqtt_client,
            &default_ha_state_config(),
        )
        .expect("Error initializing HA state collector")
    }
}

impl settings::Zigbee2Mqtt {
    async fn new_incoming_data_source(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> Z2mIncomingDataSource {
        z2m::new_incoming_data_source(
            &self.event_topic,
            &default_z2m_state_config(),
            &mut infrastructure.mqtt_client,
        )
        .await
    }
}

impl settings::Tasmota {
    async fn new_incoming_data_source(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> TasmotaIncomingDataSource {
        tasmota::new_incoming_data_source(
            &self.event_topic,
            &default_tasmota_state_config(),
            &mut infrastructure.mqtt_client,
        )
        .await
    }

    fn new_command_executor(
        &self,
        infrastructure: &Infrastructure,
    ) -> impl CommandExecutor + use<> {
        let tx = infrastructure.mqtt_client.new_publisher();
        let config = default_tasmota_command_config();
        TasmotaCommandExecutor::new(self.event_topic.clone(), config, tx)
    }
}

struct MultiCommandExecutor<A, B>
where
    A: CommandExecutor,
    B: CommandExecutor,
{
    primary: A,
    secondary: B,
}

impl<A, B> CommandExecutor for MultiCommandExecutor<A, B>
where
    A: CommandExecutor,
    B: CommandExecutor,
{
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        match self.primary.execute_command(command).await {
            Ok(true) => Ok(true),
            Ok(false) => self.secondary.execute_command(command).await,
            Err(e) => Err(e),
        }
    }
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<(Self, mpsc::Receiver<IncomingData>)> {
        let db_pool = settings.database.new_pool().await?;
        let db_listener = settings.database.new_listener().await?;
        let event_listener = AppEventListener::new(DbEventListener::new(db_listener));

        let mqtt_client = settings.mqtt.new_client();
        let database = Database::new(db_pool);

        let (incoming_data_tx, incoming_data_rx) = mpsc::channel(16);

        let infrastructure = Self {
            database,
            mqtt_client,
            event_listener,
            incoming_data_tx,
        };

        Ok((infrastructure, incoming_data_rx))
    }

    async fn subscribe_to_mqtt(
        &mut self,
        topic: &str,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<MqttInMessage>> {
        self.mqtt_client
            .subscribe(topic)
            .await
            .context("Error subscribing to MQTT topic")
    }

    fn new_command_available_listener(&self) -> Receiver<CommandAddedEvent> {
        self.event_listener.new_command_added_listener()
    }

    async fn process(self) {
        tokio::select!(
            _ = self.mqtt_client.process() => {},
            _ = self.event_listener.dispatch_events() => {},
        )
    }
}
