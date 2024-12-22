mod action;
mod action_result;
mod command_state;
mod processor;
mod resource_lock;

use std::fmt::Display;

use action_result::display_action_results;

use api::command::{Command, CommandTarget};

use crate::port::{CommandAccess, CommandExecutor, PlanningResultTracer};

pub use action::{CommandAction, ConditionalAction, ExecutableAction, ExecutionAwareAction};
pub use action_result::ActionResult;

pub use command_state::CommandState;
pub use resource_lock::Lockable;

pub async fn perform_planning<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
    result_tracer: &impl PlanningResultTracer,
) where
    G: Eq,
    A: Lockable<CommandTarget>
        + ConditionalAction<API>
        + ExecutionAwareAction<API>
        + ExecutableAction<EXE>
        + Display,
    EXE: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
{
    let action_results =
        processor::plan_and_execute(active_goals, config, api, command_processor).await;

    display_action_results(&action_results, result_tracer).await;
}
