use core::persistence::{Database, listener::DbEventListener};
use core::{HomeApi, app_event::AppEventListener};
use infrastructure::Mqtt;
use settings::Settings;

use crate::adapter::metrics_export::HomeStateMetricsExporter;
use crate::adapter::{CommandExecutorRunner, IncomingDataSourceRunner};
use crate::core::command::CommandDispatcher;
use crate::core::state::HomeStateEventEmitter;

mod adapter;
mod core;
mod home;
pub mod port;
mod settings;

struct Infrastructure {
    api: HomeApi,
    database: Database,
    event_listener: AppEventListener,
    mqtt_client: Mqtt,
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    let mut command_dispatcher = CommandDispatcher::new(&infrastructure);
    let mut home_state_event_emitter = HomeStateEventEmitter::new(&infrastructure);

    let ha_incoming_data_processing = {
        let ds = settings
            .homeassistant
            .new_incoming_data_source(&mut infrastructure)
            .await;
        IncomingDataSourceRunner::new(ds, infrastructure.api.clone())
    };
    let ha_cmd_executor = {
        let executor = settings.homeassistant.new_command_executor(&infrastructure);
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let z2m_incoming_data_processing = {
        let ds = settings.z2m.new_incoming_data_source(&mut infrastructure).await;
        IncomingDataSourceRunner::new(ds, infrastructure.api.clone())
    };
    let z2m_cmd_executor = {
        let executor = settings.z2m.new_command_executor(&infrastructure);
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let tasmota_incoming_data_processing = {
        let ds = settings.tasmota.new_incoming_data_source(&mut infrastructure).await;
        IncomingDataSourceRunner::new(ds, infrastructure.api.clone())
    };
    let tasmota_cmd_executor = {
        let executor = settings.tasmota.new_command_executor(&infrastructure);
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let energy_meter_processing = {
        let ds = adapter::energy_meter::EnergyMeter::new_incoming_data_source(&infrastructure).await;
        IncomingDataSourceRunner::new(ds, infrastructure.api.clone())
    };

    let homekit_runner = settings
        .homebridge
        .new_runner(&mut infrastructure, home_state_event_emitter.subscribe_changed())
        .await;

    let keep_planning = core::keep_on_planning(&infrastructure);

    let mut metrics_exporter = HomeStateMetricsExporter::new(home_state_event_emitter.subscribe_updated());

    let http_server_exec = {
        let http_api = infrastructure.api.clone();
        let http_database = infrastructure.database.clone();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        adapter::energy_meter::EnergyMeter::new_web_service(http_database.clone()),
                        adapter::grafana::new_routes(http_api.clone()),
                        adapter::mcp::new_routes(http_api.clone()),
                    ]
                })
                .await
                .expect("HTTP server execution failed");
        }
    };

    //try to avoid double-loading of data (other in event-dispatcher to handle the case of events
    //in between preloading and actual use)
    infrastructure
        .api
        .preload_ts_cache()
        .await
        .expect("Error preloading cache");

    tracing::info!("Starting infrastructure processing");

    let process_infrastucture = infrastructure.process();

    tracing::info!("Starting main loop");

    tokio::select!(
        _ = process_infrastucture => {},
        _ = home_state_event_emitter.run() => {},
        _ = keep_planning => {}, //TODO spawn into new thread
        _ = command_dispatcher.dispatch() => {},
        _ = energy_meter_processing.run() => {},
        _ = ha_incoming_data_processing.run() => {},
        _ = ha_cmd_executor.run() => {},
        _ = z2m_incoming_data_processing.run() => {},
        _ = z2m_cmd_executor.run() => {},
        _ = tasmota_incoming_data_processing.run() => {},
        _ = tasmota_cmd_executor.run() => {},
        _ = http_server_exec => {},
        _ = homekit_runner.run() => {},
        _ = metrics_exporter.run() => {},
    );
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        settings.monitoring.init().expect("Error initializing monitoring");

        let db_pool = settings.database.new_pool().await.expect("Error initializing database");
        let database = Database::new(db_pool);
        let api = HomeApi::new(database.clone());

        let db_listener = settings
            .database
            .new_listener()
            .await
            .expect("Error initializing database listener");
        let event_listener = AppEventListener::new(DbEventListener::new(db_listener), api.clone());

        let mqtt_client = settings.mqtt.new_client();

        Ok(Self {
            api,
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
