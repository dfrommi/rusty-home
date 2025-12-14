mod action;
mod context;
mod processor;
mod resource_lock;
mod trace;

use tokio::sync::broadcast::Receiver;
use trace::display_planning_trace;

use crate::{core::HomeApi, home::HomePlanning, home_state::StateSnapshot, trigger::TriggerClient};

pub use action::{Action, ActionEvaluationResult};
pub use trace::PlanningTrace;

pub struct PlanningRunner {
    snapshot_updated_rx: Receiver<StateSnapshot>,
    api: HomeApi,
    trigger_client: TriggerClient,
}

impl PlanningRunner {
    pub fn new(snapshot_updated_rx: Receiver<StateSnapshot>, api: HomeApi, trigger_client: TriggerClient) -> Self {
        Self {
            snapshot_updated_rx,
            api,
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
                plan_for_home(snapshot, &self.api, &self.trigger_client).await;
            }
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(snapshot: &StateSnapshot, api: &HomeApi, trigger_client: &TriggerClient) {
    tracing::info!("Start planning");
    let active_goals = HomePlanning::active_goals(snapshot.clone());
    let config = HomePlanning::config();

    let res = processor::plan_and_execute(&active_goals, config, snapshot.clone(), api, trigger_client).await;

    match res {
        Ok(res) => {
            tracing::info!("Planning done");
            display_planning_trace(&res).await;
        }

        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
