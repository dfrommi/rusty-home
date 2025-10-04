mod cool_down_when_occupied;
mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod reduce_noise_at_night;
mod user_trigger_action;

use std::fmt::Debug;
use std::fmt::Display;

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

fn action_source(action: &impl Display) -> CommandSource {
    CommandSource::System(format!("planning:{action}:start"))
}

#[derive(Debug, Clone, derive_more::Display, derive_more::From)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    InformWindowOpen(InformWindowOpen),
    ProvideAmbientTemperature(ProvideAmbientTemperature),
    IrHeaterAutoTurnOff(IrHeaterAutoTurnOff),
    ReduceNoiseAtNight(ReduceNoiseAtNight),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
    SupportVentilationWithFan(SupportVentilationWithFan),
    CoolDownWhenOccupied(CoolDownWhenOccupied),
    FollowHeatingSchedule(FollowHeatingSchedule),
}

impl Action for HomeAction {
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.evaluate(api).await,
            HomeAction::InformWindowOpen(inform_window_open) => inform_window_open.evaluate(api).await,
            HomeAction::ProvideAmbientTemperature(provide_ambient_temperature) => {
                provide_ambient_temperature.evaluate(api).await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => ir_heater_auto_turn_off.evaluate(api).await,
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => reduce_noise_at_night.evaluate(api).await,
            HomeAction::FollowDefaultSetting(follow_default_setting) => follow_default_setting.evaluate(api).await,
            HomeAction::UserTriggerAction(user_trigger_action) => user_trigger_action.evaluate(api).await,
            HomeAction::SupportVentilationWithFan(support_ventilation_with_fan) => {
                support_ventilation_with_fan.evaluate(api).await
            }
            HomeAction::CoolDownWhenOccupied(cool_down_when_occupied) => cool_down_when_occupied.evaluate(api).await,
            HomeAction::FollowHeatingSchedule(follow_heating_schedule) => follow_heating_schedule.evaluate(api).await,
        }
    }
}

//trigger and keep running until something else changes state
async fn needs_execution_for_one_shot_of_target(
    command: &Command,
    source: &CommandSource,
    oneshot_range_start: DateTime,
    api: &HomeApi,
) -> Result<bool> {
    let executions = api
        .get_all_commands_for_target(command.clone(), oneshot_range_start)
        .await?;

    let already_triggered = executions.iter().any(|e| e.source == *source && e.command == *command);

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
        Some(e) if e.source == *source && e.command == *command => e,
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
