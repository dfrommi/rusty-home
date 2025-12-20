mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod reduce_noise_at_night;
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

pub use dehumidify::Dehumidify;
pub use follow_default_setting::FollowDefaultSetting;
pub use heating::*;
pub use inform_window_open::InformWindowOpen;
pub use reduce_noise_at_night::ReduceNoiseAtNight;
pub use user_trigger_action::UserTriggerAction;

use crate::automation::planner::{Action, ActionEvaluationResult};
use crate::home_state::*;

#[derive(Debug, Clone)]
pub enum RuleResult {
    Execute(Vec<Command>),
    ExecuteTrigger(Vec<Command>, UserTriggerId),
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
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        self.snapshot
            .get(id.clone())
            .ok_or_else(|| anyhow::anyhow!("Current value for state {:?} not found", id.into()))
    }

    pub fn current<S>(&self, id: S) -> Result<S::Type>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        self.current_dp(id).map(|dp| dp.value)
    }

    pub fn latest_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.snapshot.user_trigger(target)
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

        Ok(RuleResult::Execute(vec![self.command()]))
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    InformWindowOpen(InformWindowOpen),
    ProvideAmbientTemperature(ProvideAmbientTemperature),
    ProvideLoadRoomMean(ProvideLoadRoomMean),
    AutoTurnOff(AutoTurnOff),
    ReduceNoiseAtNight(ReduceNoiseAtNight),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
    SupportVentilationWithFan(SupportVentilationWithFan),
    FollowHeatingSchedule(FollowHeatingSchedule),
    FollowTargetHeatingDemand(FollowTargetHeatingDemand),
}

impl Display for HomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext_id = self.ext_id();
        write!(f, "{}::{}", ext_id.type_name(), ext_id.variant_name())
    }
}

impl Action for HomeAction {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<ActionEvaluationResult> {
        let ext_id = self.ext_id();

        match self {
            HomeAction::Dehumidify(dehumidify) => evaluate_rule(dehumidify, ext_id, ctx),
            HomeAction::InformWindowOpen(inform_window_open) => evaluate_rule(inform_window_open, ext_id, ctx),
            HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => {
                evaluate_rule(provide_ambient_temperature, ext_id, ctx)
            }
            HomeAction::ProvideLoadRoomMean(provide_load_room_mean) => {
                evaluate_rule(provide_load_room_mean, ext_id, ctx)
            }
            HomeAction::AutoTurnOff(auto_turn_off) => evaluate_rule(auto_turn_off, ext_id, ctx),
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => evaluate_rule(reduce_noise_at_night, ext_id, ctx),
            HomeAction::FollowDefaultSetting(follow_default_setting) => {
                evaluate_rule(follow_default_setting, ext_id, ctx)
            }
            HomeAction::UserTriggerAction(user_trigger_action) => evaluate_rule(user_trigger_action, ext_id, ctx),
            HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                evaluate_rule(support_ventilation_with_fan, ext_id, ctx)
            }
            HomeAction::FollowHeatingSchedule(follow_heating_schedule) => {
                evaluate_rule(follow_heating_schedule, ext_id, ctx)
            }
            HomeAction::FollowTargetHeatingDemand(follow_target_heating_demand) => {
                evaluate_rule(follow_target_heating_demand, ext_id, ctx)
            }
        }
    }

    fn ext_id(&self) -> ExternalId {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.ext_id(),
            HomeAction::InformWindowOpen(inform_window_open) => inform_window_open.ext_id(),
            HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => provide_ambient_temperature.ext_id(),
            HomeAction::ProvideLoadRoomMean(provide_load_room_mean) => provide_load_room_mean.ext_id(),
            HomeAction::AutoTurnOff(auto_turn_off) => auto_turn_off.ext_id(),
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => reduce_noise_at_night.ext_id(),
            HomeAction::FollowDefaultSetting(follow_default_setting) => follow_default_setting.ext_id(),
            HomeAction::UserTriggerAction(user_trigger_action) => user_trigger_action.ext_id(),
            HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                support_ventilation_with_fan.ext_id()
            }
            HomeAction::FollowHeatingSchedule(follow_heating_schedule) => follow_heating_schedule.ext_id(),
            HomeAction::FollowTargetHeatingDemand(follow_target_heating_demand) => follow_target_heating_demand.ext_id(),
        }
    }
}

fn evaluate_rule(rule: &impl Rule, ext_id: ExternalId, ctx: &RuleEvaluationContext) -> Result<ActionEvaluationResult> {
    match rule.evaluate(ctx)? {
        RuleResult::Execute(commands) => Ok(ActionEvaluationResult::Execute(commands, ext_id)),
        RuleResult::ExecuteTrigger(commands, user_trigger_id) => {
            Ok(ActionEvaluationResult::ExecuteTrigger(commands, ext_id, user_trigger_id))
        }
        RuleResult::Skip => Ok(ActionEvaluationResult::Skip),
    }
}
