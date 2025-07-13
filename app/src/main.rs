use core::persistence::{Database, listener::DbEventListener};
use core::{HomeApi, app_event::AppEventListener};
use infrastructure::Mqtt;
use settings::Settings;

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
    let ha_cmd_executor = settings.homeassistant.new_command_executor(&infrastructure);

    let z2m_incoming_data_processing = settings.z2m.new_incoming_data_processor(&mut infrastructure).await;

    let tasmota_incoming_data_processing = settings.tasmota.new_incoming_data_processor(&mut infrastructure).await;
    let tasmota_cmd_executor = settings.tasmota.new_command_executor(&infrastructure);

    let energy_meter_processing = energy_meter
        .new_incoming_data_processor(
            infrastructure.database.clone(),
            infrastructure.event_listener.new_energy_reading_added_listener(),
        )
        .await;

    let hk_export_states = settings.homekit.export_state(&infrastructure);
    let hk_process_commands = settings.homekit.process_commands(&mut infrastructure).await;

    let plan_and_execute = { core::plan_and_execute(&infrastructure, ha_cmd_executor, tasmota_cmd_executor) };

    let http_server_exec = {
        let http_api = infrastructure.api.clone();
        let http_database = infrastructure.database.clone();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        adapter::energy_meter::new_web_service(http_database.clone()),
                        adapter::grafana::new_routes(http_api.clone()),
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
    let home_state_metrics_updater = core::metrics::start_home_state_metrics_updater(&infrastructure);
    let process_infrastucture = infrastructure.process();

    tracing::info!("Starting main loop");

    tokio::select!(
        _ = process_infrastucture => {},
        _ = plan_and_execute => {},
        _ = energy_meter_processing => {},
        _ = ha_incoming_data_processing => {},
        _ = z2m_incoming_data_processing => {},
        _ = tasmota_incoming_data_processing => {},
        _ = http_server_exec => {},
        _ = hk_export_states => {},
        _ = hk_process_commands => {},
        _ = home_state_metrics_updater => {},
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
