use api::command::Command;
use chrono::{DateTime, Utc};

use api::state::ChannelValue;

mod homeassistant;
pub mod persistence;

use anyhow::Result;
pub use homeassistant::HaCommandExecutor;
pub use homeassistant::HaStateCollector;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct PersistentDataPoint {
    value: ChannelValue,
    timestamp: DateTime<Utc>,
}

trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

trait StateCollector {
    async fn process(self, dp_tx: &mpsc::Sender<PersistentDataPoint>) -> anyhow::Result<()>;
}

//TODO more generic with Vec of StateCollector, but that's not possible yet because of async fn on
//trait prevents building an object. async-trait crate might help.
pub async fn collect_states(api: &persistence::BackendApi, ha_collector: HaStateCollector) {
    let mut tasks = JoinSet::new();
    let (tx, mut rx) = mpsc::channel::<PersistentDataPoint>(32);

    let ha_tx = tx.clone();
    tasks.spawn(async move {
        ha_collector
            .process(&ha_tx)
            .await
            .expect("Error processing HA events");
    });

    while let Some(dp) = rx.recv().await {
        if let Err(e) = api.add_thing_value(&dp.value, &dp.timestamp).await {
            tracing::error!("Error persisting data-point {:?}: {}", dp, e);
        }
    }

    while let Some(task) = tasks.join_next().await {
        task.unwrap();
    }
}

pub async fn execute_commands(
    api: &persistence::BackendApi,
    mut new_cmd_rx: broadcast::Receiver<()>,
    ha_executor: &HaCommandExecutor,
) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(15));
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

                //TODO find a way to handle executors in a generic way. How to iterate over traits
                //with async fn?
                let ha_res = ha_executor.execute_command(&cmd.command).await;
                handle_execution_result(cmd.id, ha_res, api).await;
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

async fn handle_execution_result(
    command_id: i64,
    res: Result<bool>,
    api: &persistence::BackendApi,
) {
    let set_state_res = match res {
        Ok(true) => api.set_command_state_success(command_id).await,
        Ok(false) => Ok(()),
        Err(e) => {
            tracing::error!("Command {} failed: {:?}", command_id, e);
            api.set_command_state_error(command_id, &e.to_string())
                .await
        }
    };

    if let Err(e) = set_state_res {
        tracing::error!("Error setting command state for {}: {}", command_id, e);
    }
}
