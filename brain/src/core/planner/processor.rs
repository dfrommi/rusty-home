use std::fmt::Display;

use api::command::CommandTarget;

use crate::{
    core::planner::action::ActionEvaluationResult,
    port::{CommandExecutionResult, CommandExecutor},
};

use super::{action::Action, resource_lock::ResourceLock, PlanningTrace};

pub async fn plan_and_execute<G, A, API, EXE>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &API,
    command_processor: &EXE,
) -> Vec<PlanningTrace>
where
    G: Eq + Display,
    A: Action<API>,
    EXE: CommandExecutor,
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

            let evaluation_result = action.evaluate(api).await.unwrap_or_else(|e| {
                tracing::warn!(
                    "Error evaluating action {}, assuming not fulfilled: {:?}",
                    action,
                    e
                );
                ActionEvaluationResult::Skip
            });

            trace.is_fulfilled = Some(!matches!(evaluation_result, ActionEvaluationResult::Skip));

            //LOCKING
            let locking_key = match &evaluation_result {
                ActionEvaluationResult::Lock(target) => Some(target.clone()),
                ActionEvaluationResult::Execute(command, _) => {
                    Some(CommandTarget::from(command.clone()))
                }
                ActionEvaluationResult::Skip => None,
            };

            match locking_key {
                Some(key) if resource_lock.is_locked(&key) => {
                    trace.locked = true;
                    action_results.push(trace);
                    continue;
                }
                Some(key) => {
                    resource_lock.lock(key);
                }
                None => {}
            }

            //EXECUTION
            if let ActionEvaluationResult::Execute(command, source) = evaluation_result {
                match command_processor.execute(command, source).await {
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
