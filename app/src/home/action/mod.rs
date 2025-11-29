mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod reduce_noise_at_night;
mod user_trigger_action;

use std::fmt::Debug;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;

use crate::core::id::ExternalId;
use crate::home::command::Command;
use crate::home::trigger::UserTriggerId;
use anyhow::Result;

pub use dehumidify::Dehumidify;
pub use follow_default_setting::FollowDefaultSetting;
pub use heating::*;
pub use inform_window_open::InformWindowOpen;
pub use reduce_noise_at_night::ReduceNoiseAtNight;
pub use user_trigger_action::UserTriggerAction;

use crate::core::HomeApi;
use crate::core::planner::Action;
use crate::core::planner::ActionEvaluationResult;
use crate::home::state::*;
use crate::port::*;

#[derive(Debug, Clone)]
pub enum RuleResult {
    Execute(Vec<Command>),
    ExecuteTrigger(Vec<Command>, UserTriggerId),
    Skip,
}

pub trait Rule {
    async fn evaluate(&self, api: &HomeApi) -> Result<RuleResult>;
}

pub trait SimpleRule {
    fn command(&self) -> Command;
    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool>;
}

impl<T: SimpleRule> Rule for T {
    async fn evaluate(&self, api: &HomeApi) -> Result<RuleResult> {
        let preconditions_fulfilled = self.preconditions_fulfilled(api).await?;

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
}

impl Display for HomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext_id = self.ext_id();
        write!(f, "{}::{}", ext_id.type_name(), ext_id.variant_name())
    }
}

impl Action for HomeAction {
    fn evaluate<'a>(
        &'a self,
        api: &'a HomeApi,
    ) -> Pin<Box<dyn Future<Output = Result<ActionEvaluationResult>> + Send + 'a>> {
        Box::pin(async move {
            let ext_id = self.ext_id();

            match self {
                HomeAction::Dehumidify(dehumidify) => evaluate_rule(dehumidify, ext_id, api).await,
                HomeAction::InformWindowOpen(inform_window_open) => {
                    evaluate_rule(inform_window_open, ext_id, api).await
                }
                HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => {
                    evaluate_rule(provide_ambient_temperature, ext_id, api).await
                }
                HomeAction::ProvideLoadRoomMean(provide_load_room_mean) => {
                    evaluate_rule(provide_load_room_mean, ext_id, api).await
                }
                HomeAction::AutoTurnOff(auto_turn_off) => evaluate_rule(auto_turn_off, ext_id, api).await,
                HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                    evaluate_rule(reduce_noise_at_night, ext_id, api).await
                }
                HomeAction::FollowDefaultSetting(follow_default_setting) => {
                    evaluate_rule(follow_default_setting, ext_id, api).await
                }
                HomeAction::UserTriggerAction(user_trigger_action) => {
                    evaluate_rule(user_trigger_action, ext_id, api).await
                }
                HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                    evaluate_rule(support_ventilation_with_fan, ext_id, api).await
                }
                HomeAction::FollowHeatingSchedule(follow_heating_schedule) => {
                    evaluate_rule(follow_heating_schedule, ext_id, api).await
                }
            }
        })
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
        }
    }
}

async fn evaluate_rule(rule: &impl Rule, ext_id: ExternalId, api: &HomeApi) -> Result<ActionEvaluationResult> {
    match rule.evaluate(api).await? {
        RuleResult::Execute(commands) => Ok(ActionEvaluationResult::Execute(commands, ext_id)),
        RuleResult::ExecuteTrigger(commands, user_trigger_id) => {
            Ok(ActionEvaluationResult::ExecuteTrigger(commands, ext_id, user_trigger_id))
        }
        RuleResult::Skip => Ok(ActionEvaluationResult::Skip),
    }
}
