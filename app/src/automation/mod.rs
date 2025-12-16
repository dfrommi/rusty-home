pub mod domain;
pub mod planner;

pub use domain::*;

use tokio::sync::broadcast::{Receiver, error::RecvError};

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

                event = self.snapshot_updated_rx.recv() => match event {
                    Ok(new_snapshot) => {
                        last_snapshot = Some(new_snapshot);
                    },
                    Err(RecvError::Closed) => {
                        tracing::error!("State snapshot receiver channel closed");
                    },
                    Err(RecvError::Lagged(count)) => {
                        tracing::warn!("State snapshot receiver lagged by {} messages", count);
                    }
                },
            };

            if let Some(ref snapshot) = last_snapshot {
                plan_for_home(snapshot, &self.command_client, &self.trigger_client).await;
            }
        }
    }
}
