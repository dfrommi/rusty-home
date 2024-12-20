use std::sync::Arc;

use actix_web::{App, HttpServer};
use api::DbEventListener;
use planning::plan_for_home;
use settings::Settings;
use sqlx::{postgres::PgListener, PgPool};
use tokio::task::JoinSet;

mod adapter;
mod planning;
pub mod port;
mod settings;
mod state;
mod support;

struct Infrastructure {
    db_pool: PgPool,
}

impl AsRef<PgPool> for Infrastructure {
    fn as_ref(&self) -> &PgPool {
        &self.db_pool
    }
}

#[tokio::main]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    //TODO
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

    let infrastructure = Arc::new(Infrastructure { db_pool });

    let event_listener = DbEventListener::new(db_listener);

    let mut planning_trigger = event_listener.new_state_value_added_listener();
    tasks.spawn({
        let api = infrastructure.clone();
        async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = timer.tick() => {},
                    _ = planning_trigger.recv() => {},
                }

                tracing::info!("Start planning");
                plan_for_home(api.as_ref()).await;
                tracing::info!("Planning done");
            }
        }
    });

    tasks.spawn({
        let mqtt_api = infrastructure.clone();
        let mqtt_sender = mqtt_client.new_publisher();
        let state_topic = settings.mqtt.base_topic_status.clone();
        let mqtt_trigger = event_listener.new_state_value_added_listener();

        async move {
            adapter::mqtt::export_state(mqtt_api.as_ref(), state_topic, mqtt_sender, mqtt_trigger)
                .await
        }
    });

    tracing::info!("Starting command processing from mqtt");
    let mqtt_command_receiver = mqtt_client
        .subscribe(format!("{}/#", &settings.mqtt.base_topic_set))
        .await
        .unwrap();
    tasks.spawn({
        let api = infrastructure.clone();
        async move {
            adapter::mqtt::process_commands(
                settings.mqtt.base_topic_set,
                mqtt_command_receiver,
                api.as_ref(),
            )
            .await
        }
    });

    tasks.spawn(async move {
        event_listener
            .dispatch_events()
            .await
            .expect("Error processing home-events")
    });

    tasks.spawn(async move { mqtt_client.process().await });

    let http_api = infrastructure.clone();
    tasks.spawn(async move {
        let http_server = HttpServer::new(move || {
            App::new().service(adapter::grafana::new_routes(http_api.clone()))
        })
        .workers(1)
        .disable_signals()
        .bind(("0.0.0.0", settings.http_server.port))
        .expect("Error configuring HTTP server");

        http_server.run().await.unwrap();
    });

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}
