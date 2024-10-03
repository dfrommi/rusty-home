use adapter::persistence::{BackendApi, BackendEventListener};
use api::command::Command;
use settings::Settings;
use sqlx::postgres::PgListener;
use std::env;
use tracing::info;

use tokio::{
    sync::{
        broadcast::Receiver,
        mpsc::{self, Sender},
    },
    task::JoinSet,
};

mod adapter;
mod error;
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

    //Migrate to broadcast when needed
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);

    info!("Start processing messages");

    info!("Starting HA-event processing");
    let ha_settings = settings.homeassistant.clone();
    let ha_event_rx = mqtt_client
        .subscribe(&ha_settings.topic_event)
        .await
        .unwrap();
    let ha_evt_api = api.clone();
    tasks.spawn(async move {
        adapter::process_ha_events(&ha_evt_api, ha_event_rx, &ha_settings)
            .await
            .unwrap();
    });

    info!("Starting HA-command processing");
    let ha_cmd_tx = mqtt_client.new_publisher();
    tasks.spawn(async move {
        adapter::process_ha_commands(cmd_rx, ha_cmd_tx, &settings.homeassistant.topic_command).await
    });

    tasks.spawn(async move { mqtt_client.process().await });

    let cmd_api = api.clone();
    let new_cmd_rx = event_listener.new_command_added_listener();
    tasks.spawn(async move { dispatch_pending_commands(&cmd_api, new_cmd_rx, cmd_tx).await });

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

pub async fn dispatch_pending_commands(
    api: &BackendApi,
    mut new_cmd_rx: Receiver<()>,
    tx: Sender<Command>,
) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut got_cmd = false;

    loop {
        //Busy loop if command was found to process as much as possible
        if !got_cmd {
            tokio::select! {
                _ = new_cmd_rx.recv() => {},
                _ = timer.tick() => {},
            };
        }

        let command = api.get_command_for_processing().await;

        match command {
            Ok(Some(cmd)) => {
                got_cmd = true;
                if let Err(e) = tx.send(cmd).await {
                    tracing::error!("Error dispatching command: {}", e);
                }
            }
            Ok(None) => {
                got_cmd = false;
            }
            Err(e) => {
                tracing::error!("Error getting pending commands: {:?}", e);
                got_cmd = false;
            }
        }
    }
}
