use std::sync::OnceLock;

use adapter::persistence::{HomeApi, HomeEventListener};
use settings::Settings;
use sqlx::postgres::PgListener;
use thing::do_plan;
use tokio::task::JoinSet;

mod adapter;
mod prelude;
mod settings;
mod support;
mod thing;

static HOME_API_INSTANCE: OnceLock<HomeApi> = OnceLock::new();
pub fn home_api() -> &'static HomeApi {
    HOME_API_INSTANCE
        .get()
        .expect("Global home-api instance accessed before initialization")
}

#[tokio::main]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    unsafe { std::env::set_var("RUST_LOG", "warn,brain=debug,support=debug") };
    tracing_subscriber::fmt::init();

    let mut tasks = JoinSet::new();

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&settings.database.url)
        .await
        .expect("Error initializing database");

    let db_listener = PgListener::connect(&settings.database.url)
        .await
        .expect("Error initializing database listener");

    let mut mqtt_client = ::support::mqtt::Mqtt::connect(
        &settings.mqtt.host,
        settings.mqtt.port,
        &settings.mqtt.client_id,
    );

    HOME_API_INSTANCE
        .set(HomeApi::new(db_pool))
        .expect("Error setting global event bus instance");

    let event_listener = HomeEventListener::new(db_listener);

    let mut planning_trigger = event_listener.new_thing_value_added_listener();
    tasks.spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = timer.tick() => {},
                _ = planning_trigger.recv() => {},
            }

            tracing::info!("Start planning");
            do_plan().await;
            tracing::info!("Planning done");
        }
    });

    let mqtt_sender = mqtt_client.new_publisher();
    let state_topic = settings.mqtt.base_topic_status.clone();
    let mqtt_trigger = event_listener.new_thing_value_added_listener();
    tasks.spawn(async move {
        adapter::mqtt::export_state(&state_topic, mqtt_sender, mqtt_trigger).await
    });

    tracing::info!("Starting command processing from mqtt");
    let mqtt_command_receiver = mqtt_client
        .subscribe(format!("{}/#", &settings.mqtt.base_topic_set))
        .await
        .unwrap();
    tasks.spawn(async move {
        adapter::mqtt::process_commands(&settings.mqtt.base_topic_set, mqtt_command_receiver).await
    });

    tasks.spawn(async move {
        event_listener
            .dispatch_events()
            .await
            .expect("Error processing home-events")
    });

    tasks.spawn(async move { mqtt_client.process().await });

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}
