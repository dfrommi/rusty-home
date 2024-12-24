use actix_web::App;
use anyhow::Context;
use api::{CommandAddedEvent, DbEventListener};
use config::{default_ha_command_config, default_ha_state_config};
use core::{CommandExecutor, IncomingDataProcessor};
use homeassistant::new_command_executor;
use settings::Settings;
use sqlx::postgres::PgListener;
use std::env;
use support::mqtt::MqttInMessage;
use tokio::sync::{broadcast::Receiver, mpsc};
use tracing::info;

use sqlx::PgPool;

mod config;
mod core;
mod energy_meter;
mod homeassistant;
mod settings;

struct Infrastructure {
    database: Database,
    event_listener: DbEventListener,
    mqtt_client: support::mqtt::Mqtt,
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
    unsafe { env::set_var("RUST_LOG", "warn,kraken=debug") };
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Error reading configuration");
    info!("Starting with settings: {:?}", settings);

    let mut infrastructure = Infrastructure::init(&settings).await.unwrap();

    let (incoming_data_tx, incoming_data_rx) = mpsc::channel(16);

    let energy_meter_processing = {
        let mut energy_meter_processor = energy_meter::new(
            infrastructure.database.clone(),
            infrastructure
                .event_listener
                .new_energy_reading_insert_listener(),
        );
        let tx = incoming_data_tx.clone();

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
        async move {
            ha_incoming_data_processor
                .process(incoming_data_tx.clone())
                .await
                .expect("Error processing HA incoming data");
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
        let ha_cmd_executor = settings.homeassistant.new_command_executor();

        async move {
            core::execute_commands(&command_repo, &ha_cmd_executor, new_cmd_available).await;
        }
    };

    //TODO embed into infrastructure, type of factory is problematic
    let http_db = infrastructure.database.clone();
    let http_server = actix_web::HttpServer::new(move || {
        App::new().service(energy_meter::new_web_service(http_db.clone()))
    })
    .workers(1)
    .disable_signals()
    .bind(("0.0.0.0", settings.http_server.port))
    .expect("Error configuring HTTP server");

    let http_server_exec = async move {
        http_server.run().await.unwrap();
    };

    let process_infrastucture = infrastructure.process();

    tokio::select!(
        _ = energy_meter_processing => {},
        _ = ha_incoming_data_processing => {},
        _ = incoming_data_persisting => {},
        _ = execute_commands => {},
        _ = http_server_exec => {},
        _ = process_infrastucture => {},
    );
}

impl settings::HomeAssitant {
    fn new_command_executor(&self) -> impl CommandExecutor {
        let http_client = homeassistant::HaRestClient::new(&self.url, &self.token);
        new_command_executor(http_client, &default_ha_command_config())
    }

    async fn new_incoming_data_processor(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl IncomingDataProcessor {
        let http_client = homeassistant::HaRestClient::new(&self.url, &self.token);
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

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&settings.database.url)
            .await
            .unwrap();

        let db_listener = PgListener::connect(&settings.database.url)
            .await
            .expect("Error initializing database listener");

        let mqtt_client = support::mqtt::Mqtt::connect(
            &settings.mqtt.host,
            settings.mqtt.port,
            &settings.mqtt.client_id,
        );

        let database = Database::new(db_pool);

        Ok(Self {
            database,
            mqtt_client,
            event_listener: DbEventListener::new(db_listener),
        })
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
