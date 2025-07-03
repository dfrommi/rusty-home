use core::{CommandExecutor, app_event::AppEventListener};
use std::future::Future;

use api::{DbEventListener, command::Command};
use core::persistence::Database;
use infrastructure::Mqtt;
use settings::Settings;

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

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    let energy_meter = adapter::energy_meter::EnergyMeter;

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

    let energy_meter_processing = energy_meter
        .new_incoming_data_processor(
            infrastructure.database.clone(),
            infrastructure
                .event_listener
                .new_energy_reading_added_listener(),
        )
        .await;

    let hk_export_states = settings.homekit.export_state(&infrastructure);
    let hk_process_commands = settings.homekit.process_commands(&mut infrastructure).await;

    let execute_commands = {
        let command_repo = infrastructure.database.clone();
        let new_cmd_available = infrastructure.event_listener.new_command_added_listener();
        let ha_cmd_executor = settings.homeassistant.new_command_executor(&infrastructure);
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
                .run_server(move || {
                    vec![
                        adapter::energy_meter::new_web_service(http_db.clone()),
                        adapter::grafana::new_routes(http_db.clone()),
                    ]
                })
                .await
                .expect("HTTP server execution failed");
        }
    };

    //try to avoid double-loading of data (other in event-dispatcher to handle the case of events
    //in between preloading and actual use)
    infrastructure
        .database
        .preload_ts_cache()
        .await
        .expect("Error preloading cache");

    let planning_exec = perform_planning(&infrastructure);
    tracing::info!("Starting infrastructure processing");
    let process_infrastucture = infrastructure.process();

    tracing::info!("Starting main loop");

    //TODO something blocking here. No execution happening
    tokio::select!(
        _ = process_infrastucture => {},
        _ = planning_exec => {},
        _ = energy_meter_processing => {},
        _ = ha_incoming_data_processing => {},
        _ = z2m_incoming_data_processing => {},
        _ = tasmota_incoming_data_processing => {},
        _ = execute_commands => {},
        _ = http_server_exec => {},
        _ = hk_export_states => {},
        _ = hk_process_commands => {},
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

    async fn process(self) {
        tokio::select!(
            _ = self.mqtt_client.process() => {},
            _ = self.event_listener.dispatch_events() => {},
        )
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
