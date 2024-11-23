use anyhow::Context;
use api::DbEventListener;
use axum::Router;
use config::{default_ha_command_config, default_ha_state_config};
use core::{CommandExecutor, NewCommandAvailablePgListener, StateCollector};
use homeassistant::new_command_executor;
use settings::Settings;
use sqlx::postgres::PgListener;
use std::env;
use support::mqtt::MqttInMessage;
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
    router: Router,
    http_listener: tokio::net::TcpListener,
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

    let (energy_meter_collector, energy_meter_router) =
        energy_meter::new(infrastructure.database.clone())
            .expect("Error initializing energy meter");
    infrastructure.add_to_router(energy_meter_router);

    let collect_states = {
        let ha_state_collector = settings
            .homeassistant
            .new_state_collector(&mut infrastructure)
            .await;
        let state_storage = infrastructure.database.clone();

        let mut multi_state_collector =
            MultiStateCollector::new(ha_state_collector, energy_meter_collector);

        async move {
            core::collect_states(&state_storage, &mut multi_state_collector)
                .await
                .unwrap();
        }
    };

    let execute_commands = {
        let command_repo = infrastructure.database.clone();
        let mut new_cmd_available = infrastructure.new_command_available_listener();
        let ha_cmd_executor = settings.homeassistant.new_command_executor();

        async move {
            core::execute_commands(&command_repo, &ha_cmd_executor, &mut new_cmd_available).await;
        }
    };

    let process_infrastucture = infrastructure.process();

    tokio::join!(collect_states, execute_commands, process_infrastucture);
}

impl settings::HomeAssitant {
    fn new_command_executor(&self) -> impl CommandExecutor {
        let http_client = homeassistant::HaRestClient::new(&self.url, &self.token);
        new_command_executor(http_client, &default_ha_command_config())
    }

    async fn new_state_collector(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl StateCollector {
        let http_client = homeassistant::HaRestClient::new(&self.url, &self.token);
        let mqtt_client = homeassistant::HaMqttClient::new(
            infrastructure
                .subscribe_to_mqtt(&self.topic_event)
                .await
                .expect("Error subscribing to MQTT topic"),
        );

        homeassistant::new_state_collector(http_client, mqtt_client, &default_ha_state_config())
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
        let http_listener =
            tokio::net::TcpListener::bind(format!("0.0.0.0:{}", settings.http_server.port))
                .await
                .unwrap();

        Ok(Self {
            database,
            mqtt_client,
            event_listener: DbEventListener::new(db_listener),
            router: Router::new(),
            http_listener,
        })
    }

    fn add_to_router(&mut self, router: Router) {
        self.router = self.router.clone().merge(router);
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
        let http = async { axum::serve(self.http_listener, self.router).await.unwrap() };

        tokio::select!(
            _ = self.mqtt_client.process() => {},
            _ = self.event_listener.dispatch_events() => {},
            _ = http => {},
        )
    }
}

//no dyn and box for traits with async fn
struct MultiStateCollector<A, B>
where
    A: StateCollector,
    B: StateCollector,
{
    state_collector_1: A,
    state_collector_2: B,
}

impl<A, B> MultiStateCollector<A, B>
where
    A: StateCollector,
    B: StateCollector,
{
    fn new(state_collector_1: A, state_collector_2: B) -> Self {
        Self {
            state_collector_1,
            state_collector_2,
        }
    }
}

impl<A, B> StateCollector for MultiStateCollector<A, B>
where
    A: StateCollector,
    B: StateCollector,
{
    async fn get_current_state(
        &self,
    ) -> anyhow::Result<Vec<support::DataPoint<api::state::ChannelValue>>> {
        let mut result = vec![];
        result.extend(self.state_collector_1.get_current_state().await?);
        result.extend(self.state_collector_2.get_current_state().await?);
        Ok(result)
    }

    async fn recv(&mut self) -> anyhow::Result<support::DataPoint<api::state::ChannelValue>> {
        tokio::select! {
            dp = self.state_collector_1.recv() => dp,
            dp = self.state_collector_2.recv() => dp,
        }
    }
}
