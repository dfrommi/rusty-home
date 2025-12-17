use infrastructure::{EventBus, Mqtt};
use settings::Settings;

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
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    let command_runner = CommandRunner::new(
        infrastructure.db_pool.clone(),
        infrastructure.mqtt_client.sender(),
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
    );

    let energy_meter_bus = EventBus::new(64);

    let device_state_runner = device_state::DeviceStateRunner::new(
        infrastructure.db_pool.clone(),
        &mut infrastructure.mqtt_client,
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.topic_event,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        energy_meter_bus.subscribe(),
        command_runner.subscribe(),
    )
    .await;

    let trigger_runner = trigger::TriggerRunner::new(infrastructure.db_pool.clone());

    let mut home_state_runner = HomeStateRunner::new(
        t!(3 hours),
        device_state_runner.subscribe(),
        trigger_runner.subscribe(),
        trigger_runner.client(),
        device_state_runner.client(),
    );

    let automation_runner =
        AutomationRunner::new(home_state_runner.subscribe(), command_runner.client(), trigger_runner.client());

    let homekit_runner = settings
        .homebridge
        .new_runner(&mut infrastructure, trigger_runner.client(), home_state_runner.subscribe())
        .await;

    let metrics_exporter = adapter::metrics_export::MetricsExportModule::new(
        settings.metrics.victoria_url.clone(),
        device_state_runner.subscribe(),
        home_state_runner.subscribe(),
        device_state_runner.client(),
    );

    let http_server_exec = {
        let http_device_state_client = device_state_runner.client();
        let http_command_client = command_runner.client();
        let energy_reading_emitter = energy_meter_bus.emitter();
        let metrics_export_api = metrics_exporter.router();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        adapter::energy_meter::EnergyMeter::new_web_service(energy_reading_emitter.clone()),
                        adapter::grafana::new_routes(http_command_client.clone(), http_device_state_client.clone()),
                        adapter::mcp::new_routes(),
                        metrics_export_api.clone().into(),
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

    tracing::info!("Starting main loop");

    tokio::spawn(async move {
        device_state_runner.run().await;
    });

    tokio::spawn(async move {
        automation_runner.run().await;
    });

    tokio::spawn(async move {
        home_state_runner.run().await;
    });

    tokio::spawn(async move {
        metrics_exporter.run().await;
    });

    tokio::spawn(async move {
        http_server_exec.await;
    });

    tokio::spawn(async move {
        tracing::info!("Starting HomeKit runner");
        homekit_runner.run().await;
    });

    infrastructure.mqtt_client.run().await;

    tracing::info!("Shutting down");
}

impl Infrastructure {
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        settings.monitoring.init().expect("Error initializing monitoring");

        let db_pool = settings.database.new_pool().await.expect("Error initializing database");

        let mqtt_client = settings.mqtt.new_client();

        Ok(Self { db_pool, mqtt_client })
    }
}
