use adapter::persistence::{BackendApi, BackendEventListener};
use settings::Settings;
use sqlx::postgres::PgListener;
use std::env;
use tracing::info;

use tokio::task::JoinSet;

mod adapter;
mod settings;

#[tokio::main]
pub async fn main() {
    unsafe { env::set_var("RUST_LOG", "warn,kraken=debug") };
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Error reading configuration");
    info!("Starting with settings: {:?}", settings);

    let mut tasks = JoinSet::new();

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&settings.database.url)
        .await
        .unwrap();

    let db_listener = PgListener::connect(&settings.database.url)
        .await
        .expect("Error initializing database listener");

    let mut mqtt_client = support::mqtt::Mqtt::connect(
        &settings.mqtt.host,
        settings.mqtt.port,
        &settings.mqtt.client_id,
    );

    let api = BackendApi::new(db_pool);
    let event_listener = BackendEventListener::new(db_listener);

    let ha_event_rx = mqtt_client
        .subscribe(&settings.homeassistant.topic_event)
        .await
        .unwrap();
    let ha_state_collector = adapter::HaStateCollector::new(
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        ha_event_rx,
    );

    let ha_cmd_executor = adapter::HaCommandExecutor::new(
        mqtt_client.new_publisher(),
        &settings.homeassistant.topic_command,
    );

    let state_collect_api = api.clone();
    tasks.spawn(
        async move { adapter::collect_states(&state_collect_api, ha_state_collector).await },
    );

    let cmd_exec_api = api.clone();
    let new_cmd_available = event_listener.new_command_added_listener();
    tasks.spawn(async move {
        adapter::execute_commands(&cmd_exec_api, new_cmd_available, &ha_cmd_executor).await
    });

    tasks.spawn(async move { mqtt_client.process().await });

    tasks.spawn(async move {
        event_listener
            .dispatch_events()
            .await
            .expect("Error processing home-events")
    });

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}
