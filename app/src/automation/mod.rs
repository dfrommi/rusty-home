pub mod domain;
pub mod planner;

pub use domain::*;

use tokio::sync::broadcast::Receiver;

use crate::{command::CommandClient, home_state::StateSnapshot, trigger::TriggerClient};

use planner::plan_for_home;

pub struct AutomationRunner {
    snapshot_updated_rx: Receiver<StateSnapshot>,
    command_client: CommandClient,
    trigger_client: TriggerClient,
}

impl AutomationRunner {
    pub fn new(
        snapshot_updated_rx: Receiver<StateSnapshot>,
        command_client: CommandClient,
        trigger_client: TriggerClient,
    ) -> Self {
        Self {
            snapshot_updated_rx,
            command_client,
            trigger_client,
        }
    }

    pub async fn run(mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));
        let mut last_snapshot: Option<StateSnapshot> = None;

        loop {
            tokio::select! {
                _ = timer.tick() => {},

                Ok(new_snapshot) = self.snapshot_updated_rx.recv() => {
                    last_snapshot = Some(new_snapshot);
                },
            };

            if let Some(ref snapshot) = last_snapshot {
                plan_for_home(snapshot, &self.command_client, &self.trigger_client).await;
            }
        }
    }
}
