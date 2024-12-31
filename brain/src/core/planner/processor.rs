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
    let evaluated_config = evaluate_actions(active_goals, config, api).await;

    let mut resource_lock: ResourceLock<CommandTarget> = ResourceLock::new();
    let mut action_results: Vec<PlanningTrace> = Vec::new();

    for (goal, action, evaluation_result) in evaluated_config {
        let mut trace = PlanningTrace::new(action, goal);
        trace.is_goal_active = active_goals.contains(goal);
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

    action_results
}

async fn evaluate_actions<'a, G, A, API>(
    active_goals: &'a [G],
    config: &'a [(G, Vec<A>)],
    api: &API,
) -> Vec<(&'a G, &'a A, ActionEvaluationResult)>
where
    G: Eq + Display,
    A: Action<API>,
{
    //Evaluate all actions in parallel for better parallelism/performance
    let tasks = config.iter().flat_map(|(goal, actions)| {
        let is_goal_active = active_goals.contains(goal);
        actions
            .iter()
            .map(|action| async move {
                //tracing::debug!("Evaluating action {}", action);
                let result = if is_goal_active {
                    evaluate_action(action, api).await.unwrap_or_else(|e| {
                        tracing::warn!(
                            "Error evaluating action {}, assuming not fulfilled: {:?}",
                            action,
                            e
                        );
                        ActionEvaluationResult::Skip
                    })
                } else {
                    ActionEvaluationResult::Skip
                };

                //tracing::debug!("Action {} evaluated as {:?}", action, result);

                (goal, action, result)
            })
            .collect::<Vec<_>>()
    });

    futures::future::join_all(tasks).await
}

#[tracing::instrument(skip_all, fields(action = action.to_string(), otel.name = format!("eval {}", action.to_string())))]
async fn evaluate_action<API, A: Action<API>>(
    action: &A,
    api: &API,
) -> anyhow::Result<ActionEvaluationResult> {
    action.evaluate(api).await
}
