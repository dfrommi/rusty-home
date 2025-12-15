mod action;
mod context;
mod processor;
mod resource_lock;
mod trace;

use trace::display_planning_trace;

use crate::{automation::HomePlanning, command::CommandClient, home_state::StateSnapshot, trigger::TriggerClient};

pub use action::{Action, ActionEvaluationResult};
pub use trace::PlanningTrace;

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(snapshot: &StateSnapshot, command_client: &CommandClient, trigger_client: &TriggerClient) {
    tracing::info!("Start planning");
    let active_goals = HomePlanning::active_goals(snapshot.clone());
    let config = HomePlanning::config();

    let res =
        processor::plan_and_execute(&active_goals, config, snapshot.clone(), command_client, trigger_client).await;

    match res {
        Ok(res) => {
            tracing::info!("Planning done");
            display_planning_trace(&res).await;
        }

        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
