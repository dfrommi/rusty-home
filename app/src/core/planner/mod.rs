mod action;
mod context;
mod processor;
mod resource_lock;
mod trace;

use trace::display_planning_trace;

use crate::{Infrastructure, core::HomeApi, home::HomePlanning};

pub use action::{Action, ActionEvaluationResult, SimpleAction};
pub use trace::{PlanningTrace, PlanningTraceStep};

pub fn keep_on_planning(infrastructure: &Infrastructure) -> impl Future<Output = ()> + use<> {
    let api = infrastructure.api.clone();
    let mut state_changed_events = infrastructure.event_listener.new_state_changed_listener();
    let mut user_trigger_events = infrastructure.event_listener.new_user_trigger_event_listener();

    async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = timer.tick() => {},
                _ = state_changed_events.recv() => {},
                _ = user_trigger_events.recv() => {},
            };

            plan_for_home(&api).await;
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(api: &HomeApi) {
    tracing::info!("Start planning");
    let active_goals = HomePlanning::active_goals(api).await;
    let config = HomePlanning::config();

    let res = processor::plan_and_execute(&active_goals, config, api).await;

    match res {
        Ok(res) => {
            tracing::info!("Planning done");
            println!("{:#?}", res);
            display_planning_trace(&res, api).await;
        }

        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
