use infrastructure::{EventBus, Mqtt};
use settings::Settings;

use crate::automation::AutomationModule;
use crate::command::CommandModule;
use crate::home_state::HomeStateModule;

mod automation;
mod command;
mod core;
mod device_state;
mod frontends;
mod home_state;
mod observability;
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

    let command_module = CommandModule::new(
        infrastructure.db_pool.clone(),
        infrastructure.mqtt_client.sender(),
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
    );

    let energy_meter_bus = EventBus::new(64);

    let device_state_module = device_state::DeviceStateModule::new(
        infrastructure.db_pool.clone(),
        &mut infrastructure.mqtt_client,
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.topic_event,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        energy_meter_bus.subscribe(),
        command_module.subscribe(),
    )
    .await;

    let trigger_module = trigger::TriggerModule::new(infrastructure.db_pool.clone());

    let home_state_module = HomeStateModule::new(
        t!(25 hours),
        device_state_module.subscribe(),
        trigger_module.subscribe(),
        trigger_module.client(),
        device_state_module.client(),
    );

    let automation_module =
        AutomationModule::new(home_state_module.subscribe(), command_module.client(), trigger_module.client());

    let homekit_module = settings
        .homebridge
        .new_runner(&mut infrastructure, trigger_module.client(), home_state_module.subscribe())
        .await;

    let observability_module = observability::ObservabilityModule::new(
        settings.metrics.victoria_url.clone(),
        device_state_module.subscribe(),
        home_state_module.subscribe(),
        device_state_module.client(),
        command_module.client(),
    );

    let http_server_exec = {
        let energy_reading_emitter = energy_meter_bus.emitter();
        let metrics_export_api = observability_module.api();

        async move {
            settings
                .http_server
                .run_server(move || {
                    vec![
                        frontends::energy_meter::EnergyMeter::new_web_service(energy_reading_emitter.clone()),
                        frontends::mcp::new_routes(),
                        metrics_export_api.routes(),
                    ]
                })
                .await
                .expect("HTTP server execution failed");
        }
    };

    tracing::info!("Starting main loop");

    tokio::spawn(async move {
        device_state_module.run().await;
    });

    tokio::spawn(async move {
        automation_module.run().await;
    });

    tokio::spawn(async move {
        home_state_module.run().await;
    });

    tokio::spawn(async move {
        observability_module.run().await;
    });

    tokio::spawn(async move {
        http_server_exec.await;
    });

    tokio::spawn(async move {
        tracing::info!("Starting HomeKit runner");
        homekit_module.run().await;
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
