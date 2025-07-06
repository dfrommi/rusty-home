use std::fmt::Display;

use crate::home::command::{Command, CommandSource, CommandTarget};
use anyhow::Result;
use infrastructure::TraceContext;
use tokio::sync::oneshot;

use crate::{
    Database, core::planner::action::ActionEvaluationResult, port::CommandExecutionResult,
};

use super::{PlanningTrace, action::Action, context::Context, resource_lock::ResourceLock};

pub async fn plan_and_execute<G, A>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &Database,
) -> Result<PlanningTrace>
where
    G: Eq + Display,
    A: Action,
{
    let (first_tx, mut prev_rx) = oneshot::channel();

    let mut contexts: Vec<Context<A>> = vec![];
    for (goal, actions) in config.iter() {
        let is_goal_active = active_goals.contains(goal);

        for action in actions {
            let (tx, rx) = oneshot::channel();
            let context = Context::new(goal, action, is_goal_active, prev_rx, tx);
            contexts.push(context);
            prev_rx = rx;
        }
    }

    first_tx
        .send(ResourceLock::new())
        .map_err(|_| anyhow::anyhow!("Error sending first resource lock to planner"))?;

    let mut tasks = vec![];
    for context in contexts {
        tasks.push(process_action(context, api));
    }

    let results: Result<Vec<Context<A>>> =
        futures::future::join_all(tasks).await.into_iter().collect();

    let steps = results?.into_iter().map(|r| r.trace).collect();
    Ok(PlanningTrace::current(steps))
}

#[tracing::instrument(
    skip_all,
    fields(action = %context.action, otel.name = %context.action),
)]
async fn process_action<'a, A>(
    mut context: Context<'a, A>,
    api: &Database,
) -> Result<Context<'a, A>>
where
    A: Action,
{
    context.trace.correlation_id = TraceContext::current_correlation_id();

    //EVALUATION
    let evaluation_result = evaluate_action(&mut context, api).await;

    //LOCKING
    let mut resource_lock = context.get_lock().await?;
    let evaluation_result = check_locked(&mut context, evaluation_result, &mut resource_lock);
    context.release_lock(resource_lock).await?;

    //EXECUTION
    if let ActionEvaluationResult::Execute(command, source) = evaluation_result {
        execute_action(&mut context, command, source, api).await;
    }

    Ok(context)
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
async fn evaluate_action<'a, A>(
    context: &mut Context<'a, A>,
    api: &Database,
) -> ActionEvaluationResult
where
    A: Action,
{
    let result = if context.goal_active {
        context.action.evaluate(api).await.unwrap_or_else(|e| {
            tracing::warn!(
                "Error evaluating action {}, assuming not fulfilled: {:?}",
                context.action,
                e
            );
            ActionEvaluationResult::Skip
        })
    } else {
        ActionEvaluationResult::Skip
    };

    context.trace.fulfilled = Some(!matches!(result, ActionEvaluationResult::Skip));

    result
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
fn check_locked<'a, A>(
    context: &mut Context<'a, A>,
    evaluation_result: ActionEvaluationResult,
    resource_lock: &mut ResourceLock<CommandTarget>,
) -> ActionEvaluationResult {
    let locking_key = match &evaluation_result {
        ActionEvaluationResult::Lock(target) => Some(target.clone()),
        ActionEvaluationResult::Execute(command, _) => Some(CommandTarget::from(command.clone())),
        ActionEvaluationResult::Skip => None,
    };

    match locking_key {
        Some(key) if resource_lock.is_locked(&key) => {
            context.trace.locked = true;
            return ActionEvaluationResult::Skip;
        }
        Some(key) => {
            resource_lock.lock(key);
        }
        None => {}
    }

    evaluation_result
}

#[tracing::instrument(skip(context, command_processor))]
async fn execute_action<'a, A>(
    context: &mut Context<'a, A>,
    command: Command,
    source: CommandSource,
    command_processor: &Database,
) where
    A: Action,
{
    match command_processor.execute(command, source).await {
        Ok(CommandExecutionResult::Triggered) => {
            tracing::info!("Action {} started", context.action);
            context.trace.triggered = Some(true);
        }
        Ok(CommandExecutionResult::Skipped) => {
            context.trace.triggered = Some(false);
        }
        Err(e) => {
            tracing::error!("Error starting action {}: {:?}", context.action, e);
        }
    }
}
