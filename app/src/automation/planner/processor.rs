use anyhow::Result;
use infrastructure::TraceContext;

use crate::automation::{HomeAction, RuleEvaluationContext, RuleResult};
use crate::command::{Command, CommandClient, CommandExecutionResult, CommandTarget};
use crate::core::{id::ExternalId, time::DateTime};
use crate::home_state::StateSnapshot;
use crate::trigger::{TriggerClient, UserTriggerId};

#[derive(Debug)]
enum PlanResult {
    Active(Option<UserTriggerId>),
    Executed(Option<UserTriggerId>),
    Skipped,
}

impl PlanResult {
    fn trigger_id(&self) -> Option<&UserTriggerId> {
        match self {
            Self::Active(trigger_id) | Self::Executed(trigger_id) => trigger_id.as_ref(),
            Self::Skipped => None,
        }
    }
}

#[tracing::instrument(name = "plan_for_home", skip_all)]
pub async fn plan_for_home(
    snapshot: &StateSnapshot,
    command_client: &CommandClient,
    trigger_client: &TriggerClient,
) -> anyhow::Result<()> {
    if let Some(correlation_id) = snapshot.correlation_id() {
        TraceContext::current().link_to(correlation_id);
    }

    tracing::info!("Start planning");
    let planner = Planner::new(snapshot, command_client, trigger_client)?;
    let result = planner.run().await;

    if let Err(error) = &result {
        TraceContext::current().set_error(error.to_string());
    }
    result?;

    tracing::info!("Planning done");
    Ok(())
}

struct Planner<'a> {
    command_client: &'a CommandClient,
    trigger_client: &'a TriggerClient,
    context: RuleEvaluationContext,
    planning_data_timestamp: DateTime,
}

impl<'a> Planner<'a> {
    fn new(
        snapshot: &StateSnapshot,
        command_client: &'a CommandClient,
        trigger_client: &'a TriggerClient,
    ) -> Result<Self> {
        let active_triggers = trigger_client
            .get_latest_triggers()?
            .into_iter()
            .filter(|t| t.is_active())
            .collect();

        Ok(Self {
            command_client,
            trigger_client,
            context: RuleEvaluationContext::new(snapshot.clone(), active_triggers),
            planning_data_timestamp: snapshot.timestamp(),
        })
    }

    async fn run(self) -> Result<()> {
        let resource_plans = crate::automation::domain::resource_plans();
        let mut active_trigger_ids = Vec::new();

        for (resource, rules) in &resource_plans {
            match self.evaluate_resource_plan(resource, rules).await {
                Ok(result) => {
                    if matches!(result, PlanResult::Executed(_)) {
                        TraceContext::current().set_ok();
                    }

                    if let Some(trigger_id) = result.trigger_id() {
                        active_trigger_ids.push(trigger_id.clone());
                    }
                }
                Err(error) => {
                    tracing::error!("Error evaluating resource plan for {}: {:?}", resource, error);
                    TraceContext::current().set_error(error.to_string());
                }
            }
        }

        self.update_triggers(active_trigger_ids)
    }

    #[tracing::instrument(
        name = "process_resource",
        skip(self, rules),
        fields(resource = %resource, otel.name = %resource)
    )]
    async fn evaluate_resource_plan(&self, resource: &CommandTarget, rules: &[HomeAction]) -> Result<PlanResult> {
        for action in rules {
            match self.process_action(resource, action).await {
                Ok(Some(result)) => {
                    if matches!(result, PlanResult::Executed(_)) {
                        TraceContext::current().set_ok();
                    }
                    return Ok(result);
                }
                Ok(None) => {}
                Err(error) => {
                    TraceContext::current().set_error(error.to_string());
                    return Err(error);
                }
            }
        }

        Ok(PlanResult::Skipped)
    }

    #[tracing::instrument(
        name = "process_action",
        skip_all,
        fields(resource = %resource, action = %action, otel.name = %action)
    )]
    async fn process_action(&self, resource: &CommandTarget, action: &HomeAction) -> Result<Option<PlanResult>> {
        let trace = TraceContext::current();
        let ext_id = action.ext_id();

        match action.evaluate(&self.context) {
            Ok(RuleResult::ExecuteDefault) => {
                trace.set_ok();
                match resource.to_default_command() {
                    Some(command) => Ok(Some(self.execute_command(None, command, ext_id).await)),
                    None => Ok(None),
                }
            }
            Ok(RuleResult::Execute(command)) => {
                trace.set_ok();
                Ok(Some(self.execute_command(None, command, ext_id).await))
            }
            Ok(RuleResult::ExecuteTrigger(command, trigger_id)) => {
                trace.set_ok();
                Ok(Some(self.execute_command(Some(trigger_id), command, ext_id).await))
            }
            Ok(RuleResult::Skip) => Ok(None),
            Err(error) => {
                tracing::error!("Error evaluating action {}: {:?}", action, error);
                trace.set_error(error.to_string());
                Err(error)
            }
        }
    }

    async fn execute_command(
        &self,
        trigger_id: Option<UserTriggerId>,
        command: Command,
        source: ExternalId,
    ) -> PlanResult {
        match self
            .command_client
            .execute(command, source, trigger_id.clone(), self.context.inner())
            .await
        {
            Ok(CommandExecutionResult::Executed) => {
                TraceContext::current().set_ok();
                PlanResult::Executed(trigger_id)
            }
            Ok(
                CommandExecutionResult::Debounced
                | CommandExecutionResult::AlreadyReflected
                | CommandExecutionResult::AlreadyExecuted,
            ) => {
                TraceContext::current().set_ok();
                PlanResult::Active(trigger_id)
            }
            Err(error) => {
                TraceContext::current().set_error(error.to_string());
                PlanResult::Active(trigger_id)
            }
        }
    }

    fn update_triggers(&self, active_trigger_ids: Vec<UserTriggerId>) -> anyhow::Result<()> {
        if !active_trigger_ids.is_empty() {
            self.trigger_client
                .set_triggers_active_from_if_unset(&active_trigger_ids)?;
        }

        self.trigger_client
            .disable_triggers_before_except(self.planning_data_timestamp, &active_trigger_ids)
            .map(|_| ())
    }
}
