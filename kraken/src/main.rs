use anyhow::Context;
use api::{DbEventListener, command::Command};
use core::{
    CommandExecutor, IncomingData, IncomingDataProcessor,
    event::{AppEventListener, CommandAddedEvent},
};
use infrastructure::MqttInMessage;
use settings::Settings;
use tokio::sync::{broadcast::Receiver, mpsc};

use sqlx::PgPool;

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

    let ha_incoming_data_processing = settings
        .homeassistant
        .new_incoming_data_processor(&mut infrastructure)
        .await;

    let z2m_incoming_data_processing = settings
        .z2m
        .new_incoming_data_processor(&mut infrastructure)
        .await;

    let tasmota_incoming_data_processing = settings
        .tasmota
        .new_incoming_data_processor(&mut infrastructure)
        .await;

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
