use anyhow::Result;
use infrastructure::TraceContext;

use crate::command::{Command, CommandClient, CommandTarget};
use crate::core::id::ExternalId;
use crate::core::time::DateTime;
use crate::home_state::StateSnapshot;
use crate::t;
use crate::trigger::{TriggerClient, UserTriggerId};

use crate::automation::{HomeAction, RuleEvaluationContext};

use super::PlanningTrace;
use super::action::ActionEvaluationResult;
use super::trace::PlanningTraceStep;

pub async fn plan_and_execute(
    resource_plans: &[(CommandTarget, Vec<HomeAction>)],
    snapshot: StateSnapshot,
    command_client: &CommandClient,
    trigger_client: &TriggerClient,
) -> Result<PlanningTrace> {
    debug_assert_eq!(
        resource_plans
            .iter()
            .map(|(k, _)| k)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        resource_plans.len(),
        "resource_plans contains duplicate CommandTarget keys"
    );

    let planning_data_timestamp = snapshot.timestamp();
    let ctx = RuleEvaluationContext::new(snapshot);

    let mut steps = Vec::new();
    let mut used_triggers = Vec::new();

    for (resource, rules) in resource_plans {
        evaluate_resource_plan(resource, rules, &ctx, command_client, &mut steps, &mut used_triggers).await?;
    }

    handle_trigger_updates(planning_data_timestamp, used_triggers, trigger_client).await?;

    Ok(PlanningTrace::new(steps))
}

async fn evaluate_resource_plan(
    resource: &CommandTarget,
    rules: &[HomeAction],
    ctx: &RuleEvaluationContext,
    command_client: &CommandClient,
    steps: &mut Vec<PlanningTraceStep>,
    used_triggers: &mut Vec<UserTriggerId>,
) -> Result<()> {
    let resource_span = tracing::info_span!("resource", %resource);
    let _enter = resource_span.enter();

    for action in rules {
        let mut trace = PlanningTraceStep::new(action, resource);
        trace.correlation_id = TraceContext::current().correlation_id();

        let result = {
            let _span = tracing::info_span!("evaluate_action", %action).entered();
            action.evaluate(ctx)
        };

        match result {
            Ok(ActionEvaluationResult::Execute(command, source)) => {
                trace.fulfilled = Some(true);
                execute_command(&mut trace, command, source, None, command_client, ctx).await;
                steps.push(trace);
                return Ok(());
            }
            Ok(ActionEvaluationResult::ExecuteTrigger(command, source, trigger_id)) => {
                trace.fulfilled = Some(true);
                used_triggers.push(trigger_id.clone());
                execute_command(&mut trace, command, source, Some(trigger_id), command_client, ctx).await;
                steps.push(trace);
                return Ok(());
            }
            Ok(ActionEvaluationResult::Skip) => {
                trace.fulfilled = Some(false);
                steps.push(trace);
            }
            Err(e) => {
                tracing::error!("Error evaluating action {}: {:?}", action, e);
                TraceContext::current().set_error(e.to_string());
                steps.push(trace);
            }
        }
    }

    Ok(())
}

async fn handle_trigger_updates(
    planning_data_timestamp: DateTime,
    used_triggers: Vec<UserTriggerId>,
    trigger_client: &TriggerClient,
) -> anyhow::Result<()> {
    if !used_triggers.is_empty() {
        trigger_client.set_triggers_active_from_if_unset(&used_triggers).await?;
    }

    trigger_client
        .disable_triggers_before_except(planning_data_timestamp, &used_triggers)
        .await
        .map(|_| ())
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

    if let Some(last_execution) = last_execution {
        let elapsed = last_execution.elapsed();

        if elapsed < t!(30 seconds) {
            tracing::trace!(
                "Command for {target} was last executed less than 30 seconds ago, waiting for state update. Skipping for now."
            );
            return Ok(false);
        }

        if let Some(min_wait) = command.min_wait_duration_between_executions()
            && elapsed < min_wait
        {
            tracing::trace!(
                "Command for {target} was last executed {:?} ago, which is less than the minimum wait duration of {:?}. Skipping for now.",
                elapsed,
                min_wait
            );
            return Ok(false);
        }
    }

    let is_reflected_in_state = command.is_reflected_in_state(ctx.inner(), command_client).await?;
    if is_reflected_in_state {
        tracing::trace!("Command for {target} is already reflected in state, skipping");
        return Ok(false);
    }

    tracing::trace!("Command for {target} should be executed");
    Ok(true)
}

async fn execute_command(
    trace: &mut PlanningTraceStep,
    command: Command,
    source: ExternalId,
    user_trigger_id: Option<UserTriggerId>,
    command_client: &CommandClient,
    ctx: &RuleEvaluationContext,
) {
    let target: CommandTarget = command.clone().into();

    match should_execute(&command, &source, command_client, ctx).await {
        Ok(true) => match command_client.execute(command, source, user_trigger_id).await {
            Ok(_) => {
                tracing::info!("Command {} executed via action {}", target, trace.action);
                trace.triggered = Some(true);
            }
            Err(e) => tracing::error!("Error executing command for {}: {:?}", target, e),
        },
        Ok(false) => {
            tracing::trace!("Skipped execution command {} via action {}", target, trace.action);
            trace.triggered = Some(false);
        }
        Err(e) => {
            tracing::error!(
                "Error checking whether command for {} via action {} should be started: {:?}",
                target,
                trace.action,
                e
            );
        }
    }
}
