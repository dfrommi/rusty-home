use std::future::Future;

use futures::future::select_all;
use infrastructure::{EventBus, Mqtt};
use tokio::task::{AbortHandle, JoinError, JoinHandle};

use crate::automation::AutomationModule;
use crate::command::CommandModule;
use crate::frontends::remote::RemoteModule;
use crate::home_state::HomeStateModule;
use crate::settings::Settings;

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

type AppTask = (&'static str, JoinHandle<()>);

#[tokio::main(flavor = "multi_thread")]
#[allow(clippy::expect_used)]
pub async fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    let mut infrastructure = Infrastructure::init(&settings)
        .await
        .expect("Error initializing infrastructure");

    let energy_meter_bus = EventBus::new(64);
    let command_event_bus = EventBus::new(64);

    let device_state_module = device_state::DeviceStateModule::new(
        infrastructure.db_pool.clone(),
        &mut infrastructure.mqtt_client,
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.topic_event,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        energy_meter_bus.subscribe(),
        command_event_bus.subscribe(),
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

    let command_module = CommandModule::new(
        command_event_bus,
        infrastructure.db_pool.clone(),
        &mut infrastructure.mqtt_client,
        &settings.tasmota.event_topic,
        &settings.z2m.event_topic,
        &settings.homeassistant.url,
        &settings.homeassistant.token,
        &settings.nuki.url,
        &settings.nuki.token,
        home_state_module.subscribe(),
    )
    .await;

    let automation_module =
        AutomationModule::new(home_state_module.subscribe(), command_module.client(), trigger_module.client());

    let homekit_module = settings
        .homebridge
        .new_runner(&mut infrastructure, trigger_module.client(), home_state_module.subscribe())
        .await;

    let remote_module = RemoteModule::new(
        &mut infrastructure.mqtt_client,
        &settings.z2m.event_topic,
        trigger_module.client(),
    )
    .await;

    let observability_module = observability::ObservabilityModule::new(
        settings.metrics.victoria_url.clone(),
        device_state_module.subscribe(),
        home_state_module.subscribe(),
        device_state_module.client(),
        home_state_module.client(),
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
                        metrics_export_api.routes(),
                    ]
                })
                .await
                .expect("HTTP server execution failed");
        }
    };

    tracing::info!("Starting main loop");

    let mut tasks = vec![
        spawn_app_task("device-state", async move {
            device_state_module.run().await;
        }),
        spawn_app_task("automation", async move {
            automation_module.run().await;
        }),
        spawn_app_task("home-state", async move {
            home_state_module.run().await;
        }),
        spawn_app_task("observability", async move {
            observability_module.run().await;
        }),
        spawn_app_task("http-server", async move {
            http_server_exec.await;
        }),
        spawn_app_task("command", async move {
            command_module.run().await;
        }),
        spawn_app_task("homekit", async move {
            tracing::info!("Starting HomeKit runner");
            homekit_module.run().await;
        }),
        spawn_app_task("remote", async move {
            tracing::info!("Starting remote runner");
            remote_module.run().await;
        }),
    ];
    let task_abort_handles = tasks
        .iter()
        .map(|(_, task_handle)| task_handle.abort_handle())
        .collect::<Vec<_>>();

    tokio::select! {
        (task_name, task_result) = wait_for_first_task_exit(&mut tasks) => {
            abort_tasks(task_abort_handles);
            handle_task_exit(task_name, task_result);
        }

        () = infrastructure.mqtt_client.run() => {
            abort_tasks(task_abort_handles);
            panic!("MQTT runner exited unexpectedly");
        }
    }
}

impl Infrastructure {
    #[allow(clippy::expect_used)]
    pub async fn init(settings: &Settings) -> anyhow::Result<Self> {
        settings.monitoring.init().expect("Error initializing monitoring");

        let db_pool = settings.database.new_pool().await.expect("Error initializing database");

        let mqtt_client = settings.mqtt.new_client();

        Ok(Self { db_pool, mqtt_client })
    }
}

fn spawn_app_task(name: &'static str, future: impl Future<Output = ()> + Send + 'static) -> AppTask {
    (name, tokio::spawn(future))
}

async fn wait_for_first_task_exit(tasks: &mut [AppTask]) -> (&'static str, Result<(), JoinError>) {
    let task_names = tasks.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    let task_handles = tasks.iter_mut().map(|(_, task_handle)| task_handle).collect::<Vec<_>>();
    let (task_result, task_index, _remaining_tasks) = select_all(task_handles).await;
    let task_name = task_names[task_index];

    (task_name, task_result)
}

fn abort_tasks(tasks: Vec<AbortHandle>) {
    for task in tasks {
        task.abort();
    }
}

fn handle_task_exit(task_name: &'static str, task_result: Result<(), JoinError>) -> ! {
    tracing::error!(task_name, "Application task exited; shutting down remaining tasks");

    match task_result {
        Ok(()) => panic!("Application task {task_name} exited unexpectedly"),
        Err(err) if err.is_panic() => {
            tracing::error!(task_name, "Application task panicked");
            std::panic::resume_unwind(err.into_panic());
        }
        Err(err) => panic!("Application task {task_name} was cancelled unexpectedly: {err}"),
    }
}
