use infrastructure::Mqtt;
use settings::Settings;
use tokio::sync::broadcast;

use crate::adapter::IncomingDataSourceRunner;
use crate::automation::AutomationRunner;
use crate::command::CommandRunner;
use crate::home_state::HomeStateRunner;

mod adapter;
mod automation;
mod command;
mod core;
mod device_state;
mod home_state;
pub mod port;
mod settings;
mod trigger;

struct Infrastructure {
    db_pool: sqlx::PgPool,
    mqtt_client: Mqtt,
    energy_reading_events: broadcast::Sender<adapter::energy_meter::EnergyReadingAddedEvent>,
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    let device_state_runner = device_state::DeviceStateRunner::new(
        infrastructure.db_pool.clone(),
        &mut infrastructure.mqtt_client,
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.topic_event,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
    )
    .await;

    let trigger_runner = trigger::TriggerRunner::new(infrastructure.db_pool.clone());
    let command_runner = CommandRunner::new(
        infrastructure.db_pool.clone(),
        infrastructure.mqtt_client.new_publisher(),
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        device_state_runner.client(),
    );

    let mut home_state_runner = HomeStateRunner::new(
        t!(3 hours),
        device_state_runner.subscribe(),
        trigger_runner.subscribe(),
        trigger_runner.client(),
        device_state_runner.client(),
    );

    let automation_runner = AutomationRunner::new(
        home_state_runner.subscribe_snapshot_updated(),
        command_runner.client(),
        trigger_runner.client(),
    );

    let energy_meter_processing = {
        let ds = adapter::energy_meter::EnergyMeter::new_incoming_data_source(&infrastructure).await;
        IncomingDataSourceRunner::new(ds, device_state_runner.client())
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
        let http_device_state_client = device_state_runner.client();
        let http_db_pool = infrastructure.db_pool.clone();

        let energy_reading_events = infrastructure.energy_reading_events.clone();
        let metrics = settings.metrics.clone();
        let http_command_client = command_runner.client();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        adapter::energy_meter::EnergyMeter::new_web_service(
                            http_db_pool.clone(),
                            energy_reading_events.clone(),
                        ),
                        adapter::grafana::new_routes(http_command_client.clone(), http_device_state_client.clone()),
                        adapter::mcp::new_routes(),
                        metrics.new_routes(),
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
        _ = device_state_runner.run() => {},
        _ = home_state_runner.run() => {},
        _ = automation_runner.run() => {},
        _ = command_runner.run_dispatcher() => {},
        _ = energy_meter_processing.run() => {},
        _ = http_server_exec => {},
        _ = homekit_runner.run() => {},
        _ = metrics_exporter.run() => {},
    );
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        settings.monitoring.init().expect("Error initializing monitoring");

        let db_pool = settings.database.new_pool().await.expect("Error initializing database");

        let mqtt_client = settings.mqtt.new_client();
        let (energy_reading_events, _) = broadcast::channel(16);

        Ok(Self {
            db_pool,
            mqtt_client,
            energy_reading_events,
        })
    }

    fn subscribe_energy_reading_events(&self) -> broadcast::Receiver<adapter::energy_meter::EnergyReadingAddedEvent> {
        self.energy_reading_events.subscribe()
    }

    async fn process(self) {
        tokio::select!(
            _ = self.mqtt_client.process() => {},
        )
    }
}
