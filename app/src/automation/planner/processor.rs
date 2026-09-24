use anyhow::Result;
use infrastructure::TraceContext;

use crate::automation::{HomeAction, RuleEvaluationContext};
use crate::command::{Command, CommandClient, CommandExecutionResult, CommandTarget};
use crate::core::{id::ExternalId, time::DateTime};
use crate::home_state::StateSnapshot;
use crate::trigger::{TriggerClient, UserTriggerId};

use super::action::ActionEvaluationResult;

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
    let result = Planner::new(snapshot, command_client, trigger_client).run().await;
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
    active_trigger_ids: Vec<UserTriggerId>,
}

impl<'a> Planner<'a> {
    fn new(snapshot: &StateSnapshot, command_client: &'a CommandClient, trigger_client: &'a TriggerClient) -> Self {
        Self {
            command_client,
            trigger_client,
            context: RuleEvaluationContext::new(snapshot.clone()),
            planning_data_timestamp: snapshot.timestamp(),
            active_trigger_ids: Vec::new(),
        }
    }

    async fn run(mut self) -> Result<()> {
        let resource_plans = crate::automation::domain::resource_plans();

        debug_assert_eq!(
            resource_plans
                .iter()
                .map(|(resource, _)| resource)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            resource_plans.len(),
            "resource_plans contains duplicate CommandTarget keys"
        );

        for (resource, rules) in &resource_plans {
            match self.evaluate_resource_plan(resource, rules).await {
                Ok(result) => {
                    if let Some(trigger_id) = result.trigger_id() {
                        self.active_trigger_ids.push(trigger_id.clone());
                    }
                }
                Err(error) => {
                    tracing::error!("Error evaluating resource plan for {}: {:?}", resource, error);
                    TraceContext::current().set_error(error.to_string());
                }
            }
        }

        self.update_triggers().await
    }

    #[tracing::instrument(
        name = "process_resource",
        skip(self, rules),
        fields(resource = %resource, otel.name = %resource)
    )]
    async fn evaluate_resource_plan(&self, resource: &CommandTarget, rules: &[HomeAction]) -> Result<PlanResult> {
        for action in rules {
            match self.process_action(resource, action).await {
                Ok(Some(result)) => return Ok(result),
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
        skip(self, action),
        fields(resource = %resource, action = %action, otel.name = %action)
    )]
    async fn process_action(&self, resource: &CommandTarget, action: &HomeAction) -> Result<Option<PlanResult>> {
        match action.evaluate(&self.context) {
            Ok(ActionEvaluationResult::Execute(command, source)) => {
                Ok(Some(self.execute_command(None, command, source).await))
            }
            Ok(ActionEvaluationResult::ExecuteTrigger(command, source, trigger_id)) => {
                Ok(Some(self.execute_command(Some(trigger_id), command, source).await))
            }
            Ok(ActionEvaluationResult::Skip) => Ok(None),
            Err(error) => {
                tracing::error!("Error evaluating action {}: {:?}", action, error);
                TraceContext::current().set_error(error.to_string());
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
        let target: CommandTarget = command.clone().into();

        match self
            .command_client
            .execute(command, source, trigger_id.clone(), self.context.inner())
            .await
        {
            Ok(CommandExecutionResult::Executed) => {
                tracing::info!("Command {} executed", target);
                TraceContext::current().set_ok();
                PlanResult::Executed(trigger_id)
            }
            Ok(CommandExecutionResult::Debounced | CommandExecutionResult::AlreadyReflected) => {
                tracing::trace!("Skipped execution command {}", target);
                TraceContext::current().set_ok();
                PlanResult::Active(trigger_id)
            }
            Err(error) => {
                tracing::error!("Error executing command for {}: {:?}", target, error);
                TraceContext::current().set_error(error.to_string());
                PlanResult::Active(trigger_id)
            }
        }
    }

    async fn update_triggers(&self) -> anyhow::Result<()> {
        if !self.active_trigger_ids.is_empty() {
            self.trigger_client
                .set_triggers_active_from_if_unset(&self.active_trigger_ids)
                .await?;
        }

        self.trigger_client
            .disable_triggers_before_except(self.planning_data_timestamp, &self.active_trigger_ids)
            .await
            .map(|_| ())
    }
}
