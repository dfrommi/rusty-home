mod action;
mod processor;
mod trace;

use trace::display_planning_trace;

use crate::{
    automation::domain::resource_plans, command::CommandClient, home_state::StateSnapshot, trigger::TriggerClient,
};

pub use action::ActionEvaluationResult;
pub use trace::PlanningTrace;

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(snapshot: &StateSnapshot, command_client: &CommandClient, trigger_client: &TriggerClient) {
    tracing::info!("Start planning");
    let plans = resource_plans();
    let res = processor::plan_and_execute(&plans, snapshot.clone(), command_client, trigger_client).await;

    match res {
        Ok(res) => {
            tracing::info!("Planning done");
            display_planning_trace(&res);
        }

        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
