mod cool_down_when_occupied;
mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod reduce_noise_at_night;
mod user_trigger_action;

use std::fmt::Debug;
use std::fmt::Display;

use crate::core::id::ExternalId;
use crate::home::command::Command;
use anyhow::Result;

use crate::core::time::DateTime;
use crate::home::command::CommandSource;
use crate::t;
pub use cool_down_when_occupied::CoolDownWhenOccupied;
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
    AutoTurnOff(AutoTurnOff),
    ReduceNoiseAtNight(ReduceNoiseAtNight),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
    SupportVentilationWithFan(SupportVentilationWithFan),
    CoolDownWhenOccupied(CoolDownWhenOccupied),
    FollowHeatingSchedule(FollowHeatingSchedule),
}

impl Display for HomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext_id = self.ext_id();
        write!(f, "{}::{}", ext_id.type_name(), ext_id.variant_name())
    }
}

impl Action for HomeAction {
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult> {
        let ext_id = self.ext_id();

        match self {
            HomeAction::Dehumidify(dehumidify) => evaluate_rule(dehumidify, ext_id, api).await,
            HomeAction::InformWindowOpen(inform_window_open) => evaluate_rule(inform_window_open, ext_id, api).await,
            HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => {
                evaluate_rule(provide_ambient_temperature, ext_id, api).await
            }
            HomeAction::AutoTurnOff(auto_turn_off) => evaluate_rule(auto_turn_off, ext_id, api).await,
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                evaluate_rule(reduce_noise_at_night, ext_id, api).await
            }
            HomeAction::FollowDefaultSetting(follow_default_setting) => {
                evaluate_rule(follow_default_setting, ext_id, api).await
            }
            HomeAction::UserTriggerAction(user_trigger_action) => evaluate_rule(user_trigger_action, ext_id, api).await,
            HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                evaluate_rule(support_ventilation_with_fan, ext_id, api).await
            }
            HomeAction::CoolDownWhenOccupied(cool_down_when_occupied) => {
                evaluate_rule(cool_down_when_occupied, ext_id, api).await
            }
            HomeAction::FollowHeatingSchedule(follow_heating_schedule) => {
                evaluate_rule(follow_heating_schedule, ext_id, api).await
            }
        }
    }

    fn ext_id(&self) -> ExternalId {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.ext_id(),
            HomeAction::InformWindowOpen(inform_window_open) => inform_window_open.ext_id(),
            HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => provide_ambient_temperature.ext_id(),
            HomeAction::AutoTurnOff(auto_turn_off) => auto_turn_off.ext_id(),
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => reduce_noise_at_night.ext_id(),
            HomeAction::FollowDefaultSetting(follow_default_setting) => follow_default_setting.ext_id(),
            HomeAction::UserTriggerAction(user_trigger_action) => user_trigger_action.ext_id(),
            HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                support_ventilation_with_fan.ext_id()
            }
            HomeAction::CoolDownWhenOccupied(cool_down_when_occupied) => cool_down_when_occupied.ext_id(),
            HomeAction::FollowHeatingSchedule(follow_heating_schedule) => follow_heating_schedule.ext_id(),
        }
    }
}

fn ext_id_as_source(ext_id: &ExternalId) -> CommandSource {
    CommandSource::System(format!("{}::{}", ext_id.type_name(), ext_id.variant_name()))
}

async fn evaluate_rule(rule: &impl Rule, ext_id: ExternalId, api: &HomeApi) -> Result<ActionEvaluationResult> {
    match rule.evaluate(api).await? {
        RuleResult::Execute(commands) => Ok(ActionEvaluationResult::ExecuteMulti(commands, ext_id_as_source(&ext_id))),
        RuleResult::Skip => Ok(ActionEvaluationResult::Skip),
    }
}

//trigger and keep running until something else changes state
async fn needs_execution_for_one_shot_of_target(
    command: &Command,
    ext_id: &ExternalId,
    oneshot_range_start: DateTime,
    api: &HomeApi,
) -> Result<bool> {
    let source = ext_id_as_source(ext_id);
    let executions = api
        .get_all_commands_for_target(command.clone(), oneshot_range_start)
        .await?;

    let already_triggered = executions.iter().any(|e| e.source == source && e.command == *command);

    //first trigger still pending -> start it
    if !already_triggered {
        tracing::trace!(
            ?command,
            since = ?oneshot_range_start,
            result = true,
            "Command was not triggered yet, starting it"
        );
        return Ok(true);
    }

    //return if something else happened after first trigger -> no longer fulfilled
    let this_as_last_execution = match executions.iter().last() {
        Some(e) if e.source == source && e.command == *command => e,
        Some(other) => {
            tracing::trace!(
                ?command,
                ?other,
                since = ?oneshot_range_start,
                result = false,
                "Superseded by other command, no longer fulfilled"
            );
            return Ok(false);
        }
        None => {
            tracing::warn!(
                ?command,
                result = false,
                "Logical error: no last execution found for command, but case should have been covered before"
            );
            return Ok(false);
        }
    };

    //cover for delay between sending command and receiving state change -> external change happened
    let just_triggered = this_as_last_execution.created > t!(30 seconds ago);
    let is_reflected_in_state = command.is_reflected_in_state(api).await?;
    let is_effectively_reflected = just_triggered || is_reflected_in_state;

    tracing::trace!(
        ?command,
        %just_triggered,
        %is_reflected_in_state,
        since = ?oneshot_range_start,
        result = %is_effectively_reflected,
        "Command {}",
        if is_effectively_reflected { "is effectively reflected" } else { "not effectively reflected" },
    );

    Ok(is_effectively_reflected)
}
