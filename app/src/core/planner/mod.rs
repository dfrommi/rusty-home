mod action;
mod context;
mod processor;
mod resource_lock;
mod trace;

use std::fmt::Display;

use trace::display_planning_trace;

use crate::Database;

pub use action::{Action, ActionEvaluationResult, SimpleAction};
pub use trace::{PlanningTrace, PlanningTraceStep};

pub async fn perform_planning<G, A>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &Database,
) -> anyhow::Result<()>
where
    G: Eq + Display,
    A: Action,
{
    let results = processor::plan_and_execute(active_goals, config, api).await?;
    display_planning_trace(&results, api).await;
    Ok(())
}
