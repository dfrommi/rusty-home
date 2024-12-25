mod action;
mod processor;
mod resource_lock;
mod trace;

use std::fmt::Display;

use trace::display_planning_trace;

use crate::port::{CommandExecutor, PlanningResultTracer};

pub use action::{Action, ActionEvaluationResult, CommandAction, ConditionalAction};
pub use trace::PlanningTrace;

pub async fn perform_planning<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
    result_tracer: &impl PlanningResultTracer,
) where
    G: Eq + Display,
    A: Action<API>,
    EXE: CommandExecutor,
{
    let results = processor::plan_and_execute(active_goals, config, api, command_processor).await;
    display_planning_trace(&results, result_tracer).await;
}
