pub mod domain;
pub mod planner;

pub use domain::*;
use infrastructure::EventListener;

use crate::{
    command::CommandClient,
    home_state::{HomeStateEvent, StateSnapshot},
    trigger::TriggerClient,
};

use planner::plan_for_home;

pub struct AutomationRunner {
    home_state_rx: EventListener<HomeStateEvent>,
    command_client: CommandClient,
    trigger_client: TriggerClient,
}

impl AutomationRunner {
    pub fn new(
        home_state_rx: EventListener<HomeStateEvent>,
        command_client: CommandClient,
        trigger_client: TriggerClient,
    ) -> Self {
        Self {
            home_state_rx,
            command_client,
            trigger_client,
        }
    }

    pub async fn run(mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));
        let mut last_snapshot: Option<StateSnapshot> = None;

        loop {
            tokio::select! {
                _ = timer.tick() => {
                    if let Some(snapshot) = &last_snapshot {
                        plan_for_home(snapshot, &self.command_client, &self.trigger_client).await;
                    }
                },

                event = self.home_state_rx.recv() => if let Some(HomeStateEvent::SnapshotUpdated(new_snapshot)) = event {
                    plan_for_home(&new_snapshot, &self.command_client, &self.trigger_client).await;
                    last_snapshot = Some(new_snapshot);
                },
            };
        }
    }
}
