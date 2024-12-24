mod action;
mod command_state;
mod processor;
mod resource_lock;
mod trace;

use std::fmt::Display;

use trace::display_planning_trace;

use api::command::{Command, CommandTarget};

use crate::port::{CommandAccess, CommandExecutor, PlanningResultTracer};

pub use action::{CommandAction, ConditionalAction, ExecutableAction, ExecutionAwareAction};
pub use trace::PlanningTrace;

pub use command_state::CommandState;
pub use resource_lock::Lockable;

pub async fn perform_planning<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
    result_tracer: &impl PlanningResultTracer,
) where
    G: Eq + Display,
    A: Lockable<CommandTarget>
        + ConditionalAction<API>
        + ExecutionAwareAction<API>
        + ExecutableAction<EXE>
        + Display,
    EXE: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
{
    let results = processor::plan_and_execute(active_goals, config, api, command_processor).await;

    display_planning_trace(&results, result_tracer).await;
}
