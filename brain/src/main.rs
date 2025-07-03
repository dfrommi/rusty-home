use core::app_event::AppEventListener;
use std::{future::Future, sync::Arc};

use api::DbEventListener;
use core::persistence::Database;
use infrastructure::{HttpServerConfig, Mqtt};
use settings::Settings;
use tokio::task::JoinSet;

mod adapter;
mod core;
mod home;
pub mod port;
mod settings;

struct Infrastructure {
    database: Database,
    event_listener: AppEventListener,
    mqtt_client: Mqtt,
}

#[tokio::main]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");
    let mut tasks = JoinSet::new();

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    //try to avoid double-loading of data (other in event-dispatcher to handle the case of events
    //in between preloading and actual use)
    infrastructure
        .database
        .preload_ts_cache()
        .await
        .expect("Error preloading cache");

    tasks.spawn(perform_planning(&infrastructure));
    tasks.spawn(settings.homekit.export_state(&infrastructure));
    tasks.spawn(settings.homekit.process_commands(&mut infrastructure).await);
    tasks.spawn(infrastructure.run_http_server(settings.http_server.clone()));
    tasks.spawn(infrastructure.process());

    while let Some(task) = tasks.join_next().await {
        let () = task.unwrap();
    }
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        settings
            .monitoring
            .init()
            .expect("Error initializing monitoring");

        let db_pool = settings
            .database
            .new_pool()
            .await
            .expect("Error initializing database");
        let database = Database::new(db_pool);

        let db_listener = settings
            .database
            .new_listener()
            .await
            .expect("Error initializing database listener");
        let event_listener =
            AppEventListener::new(DbEventListener::new(db_listener), database.clone());

        let mqtt_client = settings.mqtt.new_client();

        Ok(Self {
            database,
            event_listener,
            mqtt_client,
        })
    }

    fn run_http_server(&self, http_settings: HttpServerConfig) -> impl Future<Output = ()> + use<> {
        let http_api = Arc::new(self.database.clone());

        async move {
            http_settings
                .run_server(move || vec![adapter::grafana::new_routes(http_api.clone())])
                .await
                .expect("Error starting HTTP server");
        }
    }

    async fn process(self) {
        let (event_listener, mqtt_client) = (self.event_listener, self.mqtt_client);

        let app_event_processing = tokio::spawn(async move {
            tracing::debug!("Start dispatching events");
            event_listener
                .dispatch_events()
                .await
                .expect("Error processing events")
        });

        let mqtt_processing = tokio::spawn(async move {
            tracing::debug!("Start processing MQTT");
            mqtt_client.process().await
        });

        futures::future::select(app_event_processing, mqtt_processing).await;
    }
}

fn perform_planning(infrastructure: &Infrastructure) -> impl Future<Output = ()> + use<> {
    let api = infrastructure.database.clone();
    let mut state_changed_events = infrastructure.event_listener.new_state_changed_listener();
    let mut user_trigger_events = infrastructure
        .event_listener
        .new_user_trigger_event_listener();

    async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = timer.tick() => {},
                _ = state_changed_events.recv() => {},
                _ = user_trigger_events.recv() => {},
            };

            home::plan_for_home(&api).await;
        }
    }
}
