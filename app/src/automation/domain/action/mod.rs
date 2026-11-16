mod block_automation;
mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_device_state;
mod inform_window_open;
mod purify_air;
mod remote_turn_off;
mod user_trigger_action;

use std::collections::HashMap;
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
pub use inform_device_state::InformDeviceState;
pub use inform_window_open::InformWindowOpen;
pub use purify_air::PurifyAir;
pub use remote_turn_off::RemoteTurnOff;
pub use user_trigger_action::UserTriggerAction;

use infrastructure::TraceContext;

use crate::home_state::*;

#[derive(Debug, Clone)]
pub enum RuleResult {
    ExecuteDefault,
    Execute(Command),
    ExecuteTrigger(Command, UserTriggerId),
    Skip,
}

#[derive(Clone)]
pub struct RuleEvaluationContext {
    snapshot: StateSnapshot,
    active_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
}

impl RuleEvaluationContext {
    pub fn new(snapshot: StateSnapshot, active_triggers: Vec<UserTriggerExecution>) -> Self {
        let triggers_by_target = active_triggers
            .into_iter()
            .map(|trigger| (trigger.trigger.target(), trigger))
            .collect();

        Self {
            snapshot,
            active_triggers: triggers_by_target,
        }
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

        let result = self.active_triggers.get(&target);

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

#[derive(Debug, Clone, derive_more::From, r#macro::IdDelegation)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    InformDeviceState(InformDeviceState),
    InformWindowOpen(InformWindowOpen),
    PurifyAir(PurifyAir),
    AutoTurnOff(AutoTurnOff),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
    FollowTargetHeatingDemand(FollowTargetHeatingDemand),
    PauseHeatingForVentilation(PauseHeatingForVentilation),
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
    pub fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<RuleResult> {
        match self {
            HomeAction::Dehumidify(rule) => rule.evaluate(ctx),
            HomeAction::InformDeviceState(rule) => rule.evaluate(ctx),
            HomeAction::InformWindowOpen(rule) => rule.evaluate(ctx),
            HomeAction::PurifyAir(rule) => rule.evaluate(ctx),
            HomeAction::AutoTurnOff(rule) => rule.evaluate(ctx),
            HomeAction::FollowDefaultSetting(rule) => rule.evaluate(ctx),
            HomeAction::UserTriggerAction(rule) => rule.evaluate(ctx),
            HomeAction::FollowTargetHeatingDemand(rule) => rule.evaluate(ctx),
            HomeAction::PauseHeatingForVentilation(rule) => rule.evaluate(ctx),
            HomeAction::BlockAutomation(rule) => rule.evaluate(ctx),
            HomeAction::RemoteTurnOff(rule) => rule.evaluate(ctx),
        }
    }
}
