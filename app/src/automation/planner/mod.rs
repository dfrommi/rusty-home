mod action;
mod processor;
mod trace;

use std::collections::HashMap;

use crate::{
    command::{Command, CommandTarget},
    core::{id::ExternalId, time::DateTime},
};

use trace::display_planning_trace;

use crate::{
    automation::domain::resource_plans, command::CommandClient, home_state::StateSnapshot,
    notification::NotificationClient, trigger::TriggerClient,
};

pub use action::ActionEvaluationResult;
pub use trace::PlanningTrace;

#[derive(Debug, Clone)]
pub struct LastExecution {
    pub command: Command,
    pub source: ExternalId,
    pub created: DateTime,
}

pub type LastExecutions = HashMap<CommandTarget, LastExecution>;

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(
    snapshot: &StateSnapshot,
    command_client: &CommandClient,
    notification_client: &NotificationClient,
    trigger_client: &TriggerClient,
    last_executions: &mut LastExecutions,
) {
    tracing::info!("Start planning");
    let plans = resource_plans();
    let res = processor::plan_and_execute(
        &plans,
        snapshot.clone(),
        command_client,
        notification_client,
        trigger_client,
        last_executions,
    )
    .await;

    match res {
        Ok(res) => {
            tracing::info!("Planning done");
            display_planning_trace(&res);
        }

        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
