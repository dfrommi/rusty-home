use anyhow::Result;
use infrastructure::TraceContext;
use tracing::Instrument;

use crate::command::{Command, CommandClient, CommandExecutionResult, CommandTarget};
use crate::core::id::ExternalId;
use crate::core::time::DateTime;
use crate::home_state::StateSnapshot;
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

#[tracing::instrument(skip_all, fields(resource = %resource, otel.name = %resource))]
async fn evaluate_resource_plan(
    resource: &CommandTarget,
    rules: &[HomeAction],
    ctx: &RuleEvaluationContext,
    command_client: &CommandClient,
    steps: &mut Vec<PlanningTraceStep>,
    used_triggers: &mut Vec<UserTriggerId>,
) -> Result<()> {
    for action in rules {
        let action_span = tracing::info_span!("process_action", %action, otel.name = %action);

        // Synchronous evaluation — span guard is safe here (no .await)
        let (mut trace, result) = action_span.in_scope(|| {
            let mut trace = PlanningTraceStep::new(action, resource);
            trace.correlation_id = TraceContext::current().correlation_id();
            let result = action.evaluate(ctx);
            (trace, result)
        });

        match result {
            Ok(ActionEvaluationResult::Execute(command, source)) => {
                trace.fulfilled = Some(true);
                // Async execution — use .instrument() to avoid holding span guard across .await
                execute_command(&mut trace, command, source, None, command_client, ctx)
                    .instrument(action_span.clone())
                    .await;
                finalize_action_span(&action_span, action, &trace);
                steps.push(trace);
                return Ok(());
            }
            Ok(ActionEvaluationResult::ExecuteTrigger(command, source, trigger_id)) => {
                trace.fulfilled = Some(true);
                used_triggers.push(trigger_id.clone());
                execute_command(&mut trace, command, source, Some(trigger_id), command_client, ctx)
                    .instrument(action_span.clone())
                    .await;
                finalize_action_span(&action_span, action, &trace);
                steps.push(trace);
                return Ok(());
            }
            Ok(ActionEvaluationResult::Skip) => {
                trace.fulfilled = Some(false);
                steps.push(trace);
            }
            Err(e) => {
                action_span.in_scope(|| {
                    tracing::error!("Error evaluating action {}: {:?}", action, e);
                    TraceContext::current().set_error(e.to_string());
                });
                steps.push(trace);
            }
        }
    }

    Ok(())
}

fn finalize_action_span(span: &tracing::Span, action: &HomeAction, trace: &PlanningTraceStep) {
    span.in_scope(|| {
        TraceContext::current().set_span_name(action.to_string());
        if trace.triggered == Some(true) {
            TraceContext::current().set_ok();
        }
    });
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

#[tracing::instrument(skip_all)]
async fn execute_command(
    trace: &mut PlanningTraceStep,
    command: Command,
    source: ExternalId,
    user_trigger_id: Option<UserTriggerId>,
    command_client: &CommandClient,
    ctx: &RuleEvaluationContext,
) {
    let target: CommandTarget = command.clone().into();

    match command_client
        .execute(command, source, user_trigger_id, ctx.inner())
        .await
    {
        Ok(CommandExecutionResult::Executed) => {
            tracing::info!("Command {} executed via action {}", target, trace.action);
            trace.triggered = Some(true);
        }
        Ok(CommandExecutionResult::Debounced | CommandExecutionResult::AlreadyReflected) => {
            tracing::trace!("Skipped execution command {} via action {}", target, trace.action);
            trace.triggered = Some(false);
        }
        Err(e) => tracing::error!("Error executing command for {}: {:?}", target, e),
    }
}
