mod block_automation;
mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod remote_turn_off;
mod user_trigger_action;

use std::fmt::Debug;
use std::fmt::Display;

use crate::command::Command;
use crate::core::id::ExternalId;
use crate::core::timeseries::DataPoint;
use crate::trigger::UserTriggerExecution;
use crate::trigger::UserTriggerId;
use crate::trigger::UserTriggerTarget;
use anyhow::Result;

pub use block_automation::BlockAutomation;
pub use dehumidify::Dehumidify;
pub use follow_default_setting::FollowDefaultSetting;
pub use heating::*;
pub use inform_window_open::InformWindowOpen;
pub use remote_turn_off::RemoteTurnOff;
pub use user_trigger_action::UserTriggerAction;

use infrastructure::TraceContext;

use crate::automation::planner::ActionEvaluationResult;
use crate::home_state::*;

#[derive(Debug, Clone)]
pub enum RuleResult {
    Execute(Command),
    ExecuteTrigger(Command, UserTriggerId),
    Skip,
}

#[derive(Clone)]
pub struct RuleEvaluationContext {
    snapshot: StateSnapshot,
}

impl RuleEvaluationContext {
    pub fn new(snapshot: StateSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn inner(&self) -> &StateSnapshot {
        &self.snapshot
    }

    pub fn current_dp<S>(&self, id: S) -> Result<DataPoint<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone + Into<ExternalId>,
        S::Type: std::fmt::Display,
    {
        let ext_id: ExternalId = id.clone().into();
        let span = tracing::trace_span!(
            "get_home_state",
            otel.name = tracing::field::Empty,
            home_state = ext_id.to_string(),
            dp.value = tracing::field::Empty,
            dp.timestamp = tracing::field::Empty,
            dp.elapsed = tracing::field::Empty,
        );
        let _enter = span.enter();

        let result = self
            .snapshot
            .get(id.clone())
            .ok_or_else(|| anyhow::anyhow!("Current value for state {:?} not found", ext_id));

        if let Ok(ref dp) = result {
            span.record("otel.name", format!("{} - {}", ext_id, dp.value));
            span.record("dp.value", dp.value.to_string());
            span.record("dp.timestamp", dp.timestamp.to_iso_string());
            span.record("dp.elapsed", dp.timestamp.elapsed().to_iso_string());
            TraceContext::for_span(&span).set_ok();
        } else {
            span.record("otel.name", ext_id.to_string());
            TraceContext::for_span(&span).set_error(format!("{} not found", ext_id));
        }

        result
    }

    pub fn current<S>(&self, id: S) -> Result<S::Type>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone + Into<ExternalId>,
        S::Type: std::fmt::Display,
    {
        self.current_dp(id).map(|dp| dp.value)
    }

    pub fn latest_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        let span = tracing::trace_span!(
            "get_latest_user_trigger",
            otel.name = tracing::field::Empty,
            trigger_target = target.to_string(),
            trigger_id = tracing::field::Empty,
            trigger_timestamp = tracing::field::Empty,
            trigger_elapsed = tracing::field::Empty,
        );
        let _enter = span.enter();

        let result = self.snapshot.user_trigger(target.clone());

        span.record("otel.name", target.to_string());
        if let Some(trigger) = result {
            span.record("trigger_id", trigger.id.to_string());
            span.record("trigger_timestamp", trigger.timestamp.to_iso_string());
            span.record("trigger_elapsed", trigger.timestamp.elapsed().to_iso_string());
            TraceContext::for_span(&span).set_ok();
        }

        result
    }
}

pub trait Rule {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<RuleResult>;
}

pub trait SimpleRule {
    fn command(&self) -> Command;
    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool>;
}

impl<T: SimpleRule> Rule for T {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<RuleResult> {
        let preconditions_fulfilled = self.preconditions_fulfilled(ctx)?;

        if !preconditions_fulfilled {
            return Ok(RuleResult::Skip);
        }

        Ok(RuleResult::Execute(self.command()))
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    InformWindowOpen(InformWindowOpen),
    AutoTurnOff(AutoTurnOff),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
    SupportWithFan(SupportWithFan),
    FollowTargetHeatingDemand(FollowTargetHeatingDemand),
    BlockAutomation(BlockAutomation),
    RemoteTurnOff(RemoteTurnOff),
}

impl Display for HomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext_id = self.ext_id();
        write!(f, "{}::{}", ext_id.type_name(), ext_id.variant_name())
    }
}

impl HomeAction {
    fn as_rule(&self) -> (&dyn Rule, ExternalId) {
        match self {
            HomeAction::Dehumidify(r) => (r, r.ext_id()),
            HomeAction::InformWindowOpen(r) => (r, r.ext_id()),
            HomeAction::AutoTurnOff(r) => (r, r.ext_id()),
            HomeAction::FollowDefaultSetting(r) => (r, r.ext_id()),
            HomeAction::UserTriggerAction(r) => (r, r.ext_id()),
            HomeAction::SupportWithFan(r) => (r, r.ext_id()),
            HomeAction::FollowTargetHeatingDemand(r) => (r, r.ext_id()),
            HomeAction::BlockAutomation(r) => (r, r.ext_id()),
            HomeAction::RemoteTurnOff(r) => (r, r.ext_id()),
        }
    }

    pub fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<ActionEvaluationResult> {
        let (rule, ext_id) = self.as_rule();
        match rule.evaluate(ctx)? {
            RuleResult::Execute(command) => Ok(ActionEvaluationResult::Execute(command, ext_id)),
            RuleResult::ExecuteTrigger(command, user_trigger_id) => {
                Ok(ActionEvaluationResult::ExecuteTrigger(command, ext_id, user_trigger_id))
            }
            RuleResult::Skip => Ok(ActionEvaluationResult::Skip),
        }
    }

    pub fn ext_id(&self) -> ExternalId {
        self.as_rule().1
    }
}
