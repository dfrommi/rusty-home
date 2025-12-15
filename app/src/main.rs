use core::persistence::{Database, listener::DbEventListener};
use core::{HomeApi, app_event::AppEventListener};
use infrastructure::Mqtt;
use settings::Settings;

use crate::adapter::{CommandExecutorRunner, IncomingDataSourceRunner};
use crate::core::command::CommandDispatcher;
use crate::core::planner::PlanningRunner;
use crate::home_state::HomeStateRunner;

mod adapter;
mod core;
mod device_state;
mod home;
mod home_state;
pub mod port;
mod settings;
mod trigger;

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

    let device_state_runner = device_state::DeviceStateRunner::new(infrastructure.database.pool.clone());
    let trigger_runner = trigger::TriggerRunner::new(infrastructure.database.pool.clone());

    let mut home_state_runner = HomeStateRunner::new(
        t!(3 hours),
        infrastructure.event_listener.new_state_changed_listener(),
        infrastructure.event_listener.new_user_trigger_event_listener(),
        trigger_runner.client(),
        device_state_runner.client(),
    );

    let planning_runner = PlanningRunner::new(
        home_state_runner.subscribe_snapshot_updated(),
        infrastructure.api.clone(),
        trigger_runner.client(),
    );

    let ha_incoming_data_processing = {
        let ds = settings
            .homeassistant
            .new_incoming_data_source(&mut infrastructure)
            .await;
        IncomingDataSourceRunner::new(ds, trigger_runner.client(), device_state_runner.client())
    };
    let ha_cmd_executor = {
        let executor = settings
            .homeassistant
            .new_command_executor(&infrastructure, device_state_runner.client());
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let z2m_incoming_data_processing = {
        let ds = settings.z2m.new_incoming_data_source(&mut infrastructure).await;
        IncomingDataSourceRunner::new(ds, trigger_runner.client(), device_state_runner.client())
    };
    let z2m_cmd_executor = {
        let executor = settings.z2m.new_command_executor(&infrastructure);
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let tasmota_incoming_data_processing = {
        let ds = settings.tasmota.new_incoming_data_source(&mut infrastructure).await;
        IncomingDataSourceRunner::new(ds, trigger_runner.client(), device_state_runner.client())
    };
    let tasmota_cmd_executor = {
        let executor = settings.tasmota.new_command_executor(&infrastructure);
        CommandExecutorRunner::new(executor, command_dispatcher.subscribe(), infrastructure.api.clone())
    };

    let energy_meter_processing = {
        let ds = adapter::energy_meter::EnergyMeter::new_incoming_data_source(&infrastructure).await;
        IncomingDataSourceRunner::new(ds, trigger_runner.client(), device_state_runner.client())
    };

    let homekit_runner = settings
        .homebridge
        .new_runner(
            &mut infrastructure,
            trigger_runner.client(),
            home_state_runner.subscribe_state_changed(),
        )
        .await;

    let mut metrics_exporter = settings
        .metrics
        .new_exporter(device_state_runner.subscribe(), home_state_runner.subscribe_state_updated());

    let http_server_exec = {
        let http_api = infrastructure.api.clone();
        let http_device_state_client = device_state_runner.client();
        let http_database = infrastructure.database.clone();
        let metrics = settings.metrics.clone();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        adapter::energy_meter::EnergyMeter::new_web_service(http_database.clone()),
                        adapter::grafana::new_routes(http_api.clone(), http_device_state_client.clone()),
                        adapter::mcp::new_routes(http_api.clone()),
                        metrics.new_routes(http_api.clone()),
                    ]
                })
                .await
                .expect("HTTP server execution failed");
        }
    };

    tracing::info!("Starting state bootstrapping");
    home_state_runner
        .bootstrap_snapshot()
        .await
        .expect("Error bootstrapping state");
    tracing::info!("State bootstrapping completed");

    tracing::info!("Starting infrastructure processing");
    let process_infrastucture = infrastructure.process();

    tracing::info!("Starting main loop");

    tokio::select!(
        _ = process_infrastucture => {},
        _ = home_state_runner.run() => {},
        _ = planning_runner.run() => {},
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
