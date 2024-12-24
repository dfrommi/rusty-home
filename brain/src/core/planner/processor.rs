use std::fmt::Display;

use api::command::{Command, CommandTarget};

use crate::port::{CommandAccess, CommandExecutionResult, CommandExecutor};

use super::{
    resource_lock::{Lockable, ResourceLock},
    CommandState, ConditionalAction, ExecutableAction, PlanningTrace,
};

pub async fn plan_and_execute<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
) -> Vec<PlanningTrace>
where
    G: Eq + Display,
    A: Lockable<CommandTarget> + ConditionalAction<API> + ExecutableAction<EXE> + Display,
    EXE: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
{
    let mut resource_lock: ResourceLock<CommandTarget> = ResourceLock::new();
    let mut action_results: Vec<PlanningTrace> = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = active_goals.contains(goal);

        for action in actions {
            let mut trace = PlanningTrace::new(action, goal);
            trace.is_goal_active = is_goal_active;

            if !is_goal_active {
                action_results.push(trace);
                continue;
            }

            //Already locked -> skip
            if resource_lock.is_locked(action) {
                trace.locked = true;
                action_results.push(trace);
                continue;
            }

            let is_fulfilled = action
                .preconditions_fulfilled(api)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "Error checking preconditions of action {}, assuming not fulfilled: {:?}",
                        action,
                        e
                    );
                    false
                });

            trace.is_fulfilled = Some(is_fulfilled);

            if is_fulfilled {
                resource_lock.lock(action);

                match action.execute(command_processor).await {
                    Ok(CommandExecutionResult::Triggered) => {
                        tracing::info!("Action {} started", action);
                        trace.was_triggered = Some(true);
                    }
                    Ok(CommandExecutionResult::Skipped) => {
                        trace.was_triggered = Some(false);
                    }
                    Err(e) => {
                        tracing::error!("Error starting action {}: {:?}", action, e);
                    }
                }
            }

            action_results.push(trace);
        }
    }

    action_results
}
