use adapter::{
    persistence::{Database, NewCommandAvailablePgListener},
    HaRestClient, HaStateCollector,
};
use anyhow::Context;
use api::DbEventListener;
use config::{default_ha_command_config, default_ha_state_config};
use settings::Settings;
use sqlx::postgres::PgListener;
use std::{env, sync::Arc};
use support::mqtt::MqttInMessage;
use tracing::info;

use tokio::task::JoinSet;

mod adapter;
mod config;
pub mod domain;
pub mod port;
mod settings;

struct Infrastructure {
    database: Arc<Database>,
    event_listener: DbEventListener,
    mqtt_client: support::mqtt::Mqtt,
}

#[tokio::main]
pub async fn main() {
    unsafe { env::set_var("RUST_LOG", "warn,kraken=debug") };
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Error reading configuration");
    info!("Starting with settings: {:?}", settings);

    let mut tasks = JoinSet::new();

    let mut infrastructure = Infrastructure::init(&settings).await;
    let ha_client = HaRestClient::new(&settings.homeassistant.url, &settings.homeassistant.token);

    tasks.spawn({
        let ha_event_rx = infrastructure
            .subscribe_to_mqtt(&settings.homeassistant.topic_event)
            .await
            .expect("Error subscribing to MQTT topic");
        let mut ha_state_collector =
            HaStateCollector::new(ha_client.clone(), ha_event_rx, &default_ha_state_config());
        let state_storage = infrastructure.database.clone();

        async move {
            domain::collect_states(state_storage.as_ref(), &mut ha_state_collector)
                .await
                .unwrap();
        }
    });

    tasks.spawn({
        let command_repo = infrastructure.database.clone();
        let ha_cmd_executor =
            adapter::HaCommandExecutor::new(ha_client, &default_ha_command_config());
        let mut new_cmd_available = infrastructure.new_command_available_listener();

        async move {
            domain::execute_commands(
                command_repo.as_ref(),
                &ha_cmd_executor,
                &mut new_cmd_available,
            )
            .await;
        }
    });

    tasks.spawn(infrastructure.process());

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> Self {
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

        Self {
            database: Arc::new(Database::new(db_pool)),
            mqtt_client,
            event_listener: DbEventListener::new(db_listener, vec![api::THING_COMMAND_ADDED_EVENT]),
        }
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

    fn new_command_available_listener(&self) -> NewCommandAvailablePgListener {
        NewCommandAvailablePgListener::new(&self.event_listener)
            .expect("Error initializing database listener")
    }

    async fn process(self) {
        tokio::select!(
            _ = self.mqtt_client.process() => {},
            _ = self.event_listener.dispatch_events() => {},
        )
    }
}
