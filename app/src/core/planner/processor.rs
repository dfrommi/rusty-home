use std::fmt::Display;

use crate::{
    home::command::{Command, CommandSource, CommandTarget},
    t,
};
use anyhow::Result;
use infrastructure::TraceContext;
use tokio::sync::oneshot;

use crate::core::{HomeApi, planner::action::ActionEvaluationResult};

use super::{PlanningTrace, action::Action, context::Context, resource_lock::ResourceLock};

pub async fn plan_and_execute<G, A>(active_goals: &[G], config: &[(G, Vec<A>)], api: &HomeApi) -> Result<PlanningTrace>
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

    let results: Result<Vec<Context<A>>> = futures::future::join_all(tasks).await.into_iter().collect();

    let steps = results?.into_iter().map(|r| r.trace).collect();
    Ok(PlanningTrace::current(steps))
}

#[tracing::instrument(
    skip_all,
    fields(action = %context.action, otel.name = %context.action),
)]
async fn process_action<'a, A>(mut context: Context<'a, A>, api: &HomeApi) -> Result<Context<'a, A>>
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
    match evaluation_result {
        ActionEvaluationResult::Execute(command, source) => execute_action(&mut context, command, source, api).await,
        ActionEvaluationResult::ExecuteMulti(commands, source) => {
            for command in commands {
                execute_action(&mut context, command, source.clone(), api).await;
            }
        }
        ActionEvaluationResult::Lock(_) | ActionEvaluationResult::Skip => {}
    }

    Ok(context)
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
async fn evaluate_action<'a, A>(context: &mut Context<'a, A>, api: &HomeApi) -> ActionEvaluationResult
where
    A: Action,
{
    let mut result = if context.goal_active {
        context.action.evaluate(api).await.unwrap_or_else(|e| {
            tracing::warn!("Error evaluating action {}, assuming not fulfilled: {:?}", context.action, e);
            ActionEvaluationResult::Skip
        })
    } else {
        tracing::trace!("Goal {} not active, skipping action {}", context.trace.goal, context.action);
        ActionEvaluationResult::Skip
    };

    //Treat empty result as skipped to prevent further checks for empty
    if let ActionEvaluationResult::ExecuteMulti(commands, _) = &result
        && commands.is_empty()
    {
        tracing::warn!("Received empty commands list from action {}. Skipping.", context.action);
        result = ActionEvaluationResult::Skip
    }

    context.trace.fulfilled = Some(!matches!(result, ActionEvaluationResult::Skip));

    result
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
fn check_locked<'a, A>(
    context: &mut Context<'a, A>,
    evaluation_result: ActionEvaluationResult,
    resource_lock: &mut ResourceLock<CommandTarget>,
) -> ActionEvaluationResult {
    let locking_keys = match &evaluation_result {
        ActionEvaluationResult::Lock(target) => vec![target.clone()],
        ActionEvaluationResult::Execute(command, _) => vec![CommandTarget::from(command.clone())],
        ActionEvaluationResult::ExecuteMulti(commands, _) => commands
            .iter()
            .map(|command| CommandTarget::from(command.clone()))
            .collect(),
        ActionEvaluationResult::Skip => vec![],
    };

    //only succeed if all commands can be locked. Partial execution will most likely lead to
    //unwanted result
    if locking_keys.iter().any(|key| resource_lock.is_locked(key)) {
        context.trace.locked = true;
        return ActionEvaluationResult::Skip;
    }

    for key in locking_keys {
        resource_lock.lock(key);
    }

    evaluation_result
}

#[tracing::instrument(skip(api))]
async fn should_execute(
    command: Command,
    source: crate::home::command::CommandSource,
    api: &HomeApi,
) -> anyhow::Result<bool> {
    let target: CommandTarget = command.clone().into();
    let last_execution = api
        .get_latest_command(target.clone(), t!(48 hours ago))
        .await?
        .filter(|e| e.source == source && e.command == command)
        .map(|e| e.created);

    let was_just_executed = last_execution.is_some_and(|dt| dt > t!(30 seconds ago));

    if was_just_executed {
        tracing::trace!("Command for {target} was just executed, skipping");
        return Ok(false);
    }

    let is_reflected_in_state = command.is_reflected_in_state(api).await?;
    if is_reflected_in_state {
        tracing::trace!("Command for {target} is already reflected in state, skipping");
        return Ok(false);
    }

    tracing::trace!("Command for {target} should be executed");
    Ok(true)
}

#[tracing::instrument(skip(context, api))]
async fn execute_action<'a, A>(context: &mut Context<'a, A>, command: Command, source: CommandSource, api: &HomeApi)
where
    A: Action,
{
    let target: CommandTarget = command.clone().into();

    match should_execute(command.clone(), source.clone(), api).await {
        Ok(true) => match api.save_command(command, source).await {
            Ok(_) => {
                tracing::info!("Started command {} via action {}", target, context.action);
                context.trace.triggered = Some(true);
            }
            Err(e) => tracing::error!("Error saving command for {}: {:?}", target, e),
        },
        Ok(false) => {
            tracing::trace!("Skipped execution command {} via action {}", target, context.action);
            context.trace.triggered = Some(false);
        }
        Err(e) => {
            tracing::error!(
                "Error checking whether command for {} via action {} should be started: {:?}",
                target,
                context.action,
                e
            );
        }
    }
}
