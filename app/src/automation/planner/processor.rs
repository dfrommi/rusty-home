use std::fmt::Display;

use anyhow::Result;
use infrastructure::TraceContext;
use tokio::{sync::oneshot, task::yield_now};
use tracing::Instrument;

use crate::command::{Command, CommandClient, CommandTarget};
use crate::core::id::ExternalId;
use crate::core::time::DateTime;
use crate::home_state::StateSnapshot;
use crate::t;
use crate::trigger::{TriggerClient, UserTriggerId};

use crate::automation::RuleEvaluationContext;

use super::{ActionEvaluationResult, PlanningTrace, action::Action, context::Context, resource_lock::ResourceLock};

pub async fn plan_and_execute<G, A>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    snapshot: StateSnapshot,
    command_client: &CommandClient,
    trigger_client: &TriggerClient,
) -> Result<PlanningTrace>
where
    G: Eq + Display,
    A: Action + Clone + Send + Sync + 'static,
{
    let planning_start = t!(now);
    let rule_ctx = RuleEvaluationContext::new(snapshot);

    let (first_tx, mut prev_rx) = oneshot::channel();
    let mut handles = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = active_goals.contains(goal);
        let goal_name = goal.to_string();
        let goal_span =
            tracing::info_span!("planning goal", %goal_name, otel.name = %goal_name, goal_active = is_goal_active);
        let _enter = goal_span.enter();

        for action in actions {
            let (tx, rx) = oneshot::channel();
            let context = Context::new(goal, action.clone(), is_goal_active, prev_rx, tx);
            prev_rx = rx;

            let command_client = command_client.clone();
            let rule_ctx = rule_ctx.clone();
            handles.push(tokio::spawn(
                async move { process_action(context, rule_ctx, command_client).await }.instrument(goal_span.clone()),
            ));
        }
    }

    first_tx
        .send(ResourceLock::new())
        .map_err(|_| anyhow::anyhow!("Error sending first resource lock to planner"))?;

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        let context = handle
            .await
            .map_err(|e| anyhow::anyhow!("Planning task failed: {:?}", e))??;
        results.push(context);
    }

    let used_triggers = results
        .iter()
        .filter_map(|c| c.user_trigger_id.clone())
        .collect::<Vec<UserTriggerId>>();
    cancel_unused_triggers(planning_start, used_triggers, trigger_client).await?;

    let steps = results.into_iter().map(|r| r.trace).collect();
    Ok(PlanningTrace::current(steps))
}

async fn cancel_unused_triggers(
    planning_start: DateTime,
    used_triggers: Vec<UserTriggerId>,
    trigger_client: &TriggerClient,
) -> anyhow::Result<()> {
    trigger_client
        .disable_triggers_before_except(planning_start, &used_triggers)
        .await
        .map(|_| ())
}

#[tracing::instrument(
    skip_all,
    fields(action = %context.action, otel.name),
)]
async fn process_action<A: Action>(
    mut context: Context<A>,
    ctx: RuleEvaluationContext,
    command_client: CommandClient,
) -> Result<Context<A>> {
    context.trace.correlation_id = TraceContext::current_correlation_id();

    //EVALUATION
    let evaluation_result = evaluate_action(&mut context, &ctx).await;
    // Yield so other actions can start evaluating before we attempt to acquire the lock.
    yield_now().await;

    //LOCKING
    let mut resource_lock = context.get_lock().await?;
    let evaluation_result = check_locked(&mut context, evaluation_result, &mut resource_lock);
    context.release_lock(resource_lock).await?;
    // Allow the next action to pick up the lock before we start executing commands.
    yield_now().await;

    //EXECUTION
    match evaluation_result {
        ActionEvaluationResult::Execute(commands, source) => {
            for command in commands {
                execute_action(&mut context, command, source.clone(), None, &command_client, &ctx).await;
            }
        }
        ActionEvaluationResult::ExecuteTrigger(commands, source, user_trigger_id) => {
            for command in commands {
                execute_action(
                    &mut context,
                    command,
                    source.clone(),
                    Some(user_trigger_id.clone()),
                    &command_client,
                    &ctx,
                )
                .await;
            }
        }
        ActionEvaluationResult::Skip => {}
    }

    if context.trace.locked {
        TraceContext::set_current_span_name(format!("⏸ {}", context.action));
    } else if context.trace.triggered == Some(true) {
        TraceContext::set_current_span_name(format!("▶ {}", context.action));
    } else if context.trace.fulfilled == Some(true) {
        TraceContext::set_current_span_name(format!("▷ {}", context.action));
    } else {
        TraceContext::set_current_span_name(format!("{}", context.action));
    }

    Ok(context)
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
async fn evaluate_action<A: Action>(context: &mut Context<A>, ctx: &RuleEvaluationContext) -> ActionEvaluationResult {
    let mut result = if context.goal_active {
        context.action.evaluate(ctx).unwrap_or_else(|e| {
            tracing::warn!("Error evaluating action {}, assuming not fulfilled: {:?}", context.action, e);
            ActionEvaluationResult::Skip
        })
    } else {
        tracing::trace!("Goal {} not active, skipping action {}", context.trace.goal, context.action);
        ActionEvaluationResult::Skip
    };

    //Treat empty result as skipped to prevent further checks for empty
    match &result {
        ActionEvaluationResult::Execute(commands, _) | ActionEvaluationResult::ExecuteTrigger(commands, _, _)
            if commands.is_empty() =>
        {
            tracing::warn!("Received empty commands list from action {}. Skipping.", context.action);
            result = ActionEvaluationResult::Skip
        }
        _ => {}
    }

    context.trace.fulfilled = Some(!matches!(result, ActionEvaluationResult::Skip));

    result
}

#[tracing::instrument(ret(level = tracing::Level::TRACE), skip_all)]
fn check_locked<A>(
    context: &mut Context<A>,
    evaluation_result: ActionEvaluationResult,
    resource_lock: &mut ResourceLock<CommandTarget>,
) -> ActionEvaluationResult {
    let locking_keys = match &evaluation_result {
        ActionEvaluationResult::Execute(commands, _) | ActionEvaluationResult::ExecuteTrigger(commands, _, _) => {
            commands
                .iter()
                .map(|command| CommandTarget::from(command.clone()))
                .collect()
        }
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

#[tracing::instrument(skip(command_client, ctx))]
async fn should_execute(
    command: &Command,
    source: &ExternalId,
    command_client: &CommandClient,
    ctx: &RuleEvaluationContext,
) -> anyhow::Result<bool> {
    let target: CommandTarget = command.clone().into();
    let last_execution = command_client
        .get_latest_command(target.clone(), t!(48 hours ago))
        .await?
        .filter(|e| e.source == *source && e.command == *command)
        .map(|e| e.created);

    let was_just_executed = last_execution.is_some_and(|dt| dt > t!(30 seconds ago));

    if was_just_executed {
        tracing::trace!("Command for {target} was just executed, skipping");
        return Ok(false);
    }

    let is_reflected_in_state = command.is_reflected_in_state(ctx.inner(), command_client).await?;
    if is_reflected_in_state {
        tracing::trace!("Command for {target} is already reflected in state, skipping");
        return Ok(false);
    }

    tracing::trace!("Command for {target} should be executed");
    Ok(true)
}

#[tracing::instrument(skip(context, command_client, ctx))]
async fn execute_action<A: Action>(
    context: &mut Context<A>,
    command: Command,
    source: ExternalId,
    user_trigger_id: Option<UserTriggerId>,
    command_client: &CommandClient,
    ctx: &RuleEvaluationContext,
) {
    let target: CommandTarget = command.clone().into();

    context.user_trigger_id = user_trigger_id.clone();

    match should_execute(&command, &source, command_client, ctx).await {
        Ok(true) => match command_client.execute(command, source, user_trigger_id).await {
            Ok(_) => {
                tracing::info!("Command {} executed via action {}", target, context.action);
                context.trace.triggered = Some(true);
            }
            Err(e) => tracing::error!("Error executing command for {}: {:?}", target, e),
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
