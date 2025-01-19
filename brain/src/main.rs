use core::event::AppEventListener;
use std::sync::Arc;

use actix_web::{dev::Service, App, HttpServer};
use adapter::persistence::Database;
use api::DbEventListener;
use monitoring::Monitoring;
use settings::Settings;
use sqlx::postgres::PgListener;
use tokio::task::JoinSet;

mod adapter;
mod core;
mod home;
pub mod port;
mod settings;
mod support;

#[tokio::main]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");
    let mut tasks = JoinSet::new();

    let mut _monitoring =
        Monitoring::init(&settings.monitoring).expect("Error initializing monitoring");

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect(&settings.database.url)
        .await
        .expect("Error initializing database");
    let database = Database::new(db_pool);

    let db_listener = PgListener::connect(&settings.database.url)
        .await
        .expect("Error initializing database listener");
    let event_listener = AppEventListener::new(DbEventListener::new(db_listener), database.clone());

    let mut mqtt_client = ::support::mqtt::Mqtt::connect(
        &settings.mqtt.host,
        settings.mqtt.port,
        &settings.mqtt.client_id,
    );

    //try to avoid double-loading of data (other in event-dispatcher to handle the case of events
    //in between preloading and actual use)
    database
        .preload_ts_cache()
        .await
        .expect("Error preloading cache");

    tasks.spawn({
        let api = database.clone();
        let mut state_changed_events = event_listener.new_state_changed_listener();
        let mut user_trigger_events = event_listener.new_user_trigger_event_listener();
        async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = timer.tick() => {},
                    _ = state_changed_events.recv() => {},
                    _ = user_trigger_events.recv() => {},
                };

                home::plan_for_home(&api, &api, &api).await;
            }
        }
    });

    tasks.spawn({
        let mqtt_api = database.clone();
        let mqtt_sender = mqtt_client.new_publisher();
        let state_topic = settings.mqtt.base_topic_status.clone();
        let mqtt_trigger = event_listener.new_state_changed_listener();

        async move {
            adapter::mqtt::export_state(&mqtt_api, state_topic, mqtt_sender, mqtt_trigger).await
        }
    });

    tracing::info!("Starting command processing from mqtt");
    let mqtt_command_receiver = mqtt_client
        .subscribe(format!("{}/#", &settings.mqtt.base_topic_set))
        .await
        .unwrap();
    tasks.spawn({
        let api = database.clone();
        async move {
            adapter::mqtt::process_commands(
                settings.mqtt.base_topic_set,
                mqtt_command_receiver,
                &api,
            )
            .await
        }
    });

    tasks.spawn(async move {
        tracing::debug!("Start dispatching events");
        event_listener
            .dispatch_events()
            .await
            .expect("Error processing events")
    });

    tasks.spawn(async move { mqtt_client.process().await });

    let http_api = Arc::new(database.clone());
    tasks.spawn(async move {
        let http_server = HttpServer::new(move || {
            App::new()
                .wrap(tracing_actix_web::TracingLogger::default())
                .service(adapter::grafana::new_routes(http_api.clone()))
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
