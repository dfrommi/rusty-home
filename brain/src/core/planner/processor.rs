use std::fmt::Display;

use api::command::{Command, CommandTarget};
use support::t;

use crate::port::{CommandAccess, CommandExecutionResult, CommandExecutor};

use super::{
    action_result::ActionResult,
    resource_lock::{Lockable, ResourceLock},
    CommandState, ConditionalAction, ExecutableAction, ExecutionAwareAction,
};

pub async fn plan_and_execute<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
) -> Vec<ActionResult>
where
    G: Eq,
    A: Lockable<CommandTarget>
        + ConditionalAction<API>
        + ExecutionAwareAction<API>
        + ExecutableAction<EXE>
        + Display,
    EXE: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
{
    let mut resource_lock: ResourceLock<CommandTarget> = ResourceLock::new();
    let mut action_results: Vec<ActionResult> = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = active_goals.contains(goal);

        for action in actions {
            let mut result = ActionResult::new(action);
            result.is_goal_active = is_goal_active;

            if !is_goal_active {
                action_results.push(result);
                continue;
            }

            //Already locked -> skip
            if resource_lock.is_locked(action) {
                result.locked = true;
                action_results.push(result);
                continue;
            }

            let is_fulfilled = is_fulfilled_or_just_started(action, api).await;
            result.is_fulfilled = Some(is_fulfilled);

            if is_fulfilled {
                result.should_be_started = true;
                resource_lock.lock(action);

                tracing::info!("Starting action {}", action);
                match action.execute(command_processor).await {
                    Ok(CommandExecutionResult::Triggered) => {
                        tracing::info!("Action {} started", action);
                    }
                    Ok(CommandExecutionResult::Skipped) => {
                        result.is_running = Some(true);
                    }
                    Err(e) => {
                        tracing::error!("Error starting action {}: {:?}", action, e);
                    }
                }
            }

            action_results.push(result);
        }
    }

    action_results
}

async fn is_fulfilled_or_just_started<A, API>(action: &A, api: &API) -> bool
where
    A: ConditionalAction<API> + ExecutionAwareAction<API> + Display,
{
    macro_rules! unwrap_or_warn {
        ($e:expr, $default:expr, $msg:literal) => {
            $e.unwrap_or_else(|e| {
                tracing::warn!($msg, action, e);
                $default
            })
        };
    }

    let was_just_now_triggered = unwrap_or_warn!(
        action
            .was_latest_execution_for_target_since(t!(30 seconds ago), api)
            .await,
        false,
        "Error getting latest exexcution of action {}, assuming not running: {:?}"
    );

    if was_just_now_triggered {
        return true;
    }

    unwrap_or_warn!(
        action.preconditions_fulfilled(api).await,
        false,
        "Error checking preconditions of action {}, assuming not fulfilled: {:?}"
    )
}
