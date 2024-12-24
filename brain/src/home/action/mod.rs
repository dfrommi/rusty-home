mod dehumidify;
mod follow_default_setting;
mod heating;
mod inform_window_open;
mod keep_user_override;
mod reduce_noise_at_night;
mod request_closing_window;
mod save_tv_energy;
mod user_trigger_action;

use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Result;
use api::command::Command;

use api::command::CommandSource;
use api::command::SetHeating;
use api::state::ExternalAutoControl;
use api::state::Powered;
use api::state::RelativeHumidity;
use api::state::SetPoint;
pub use dehumidify::Dehumidify;
pub use follow_default_setting::FollowDefaultSetting;
pub use heating::*;
pub use inform_window_open::InformWindowOpen;
pub use keep_user_override::KeepUserOverride;
pub use reduce_noise_at_night::ReduceNoiseAtNight;
pub use request_closing_window::RequestClosingWindow;
pub use save_tv_energy::SaveTvEnergy;
pub use user_trigger_action::UserTriggerAction;

use crate::core::planner::Action;
use crate::core::planner::ActionEvaluationResult;
use crate::core::service::CommandState;
use crate::home::state::*;
use crate::port::*;

fn action_source(action: &impl Display) -> CommandSource {
    CommandSource::System(format!("planning:{}:start", action))
}

#[derive(Debug, Clone, derive_more::Display, derive_more::From)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    InformWindowOpen(InformWindowOpen),
    NoHeatingDuringVentilation(NoHeatingDuringVentilation),
    NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease),
    IrHeaterAutoTurnOff(IrHeaterAutoTurnOff),
    KeepUserOverride(KeepUserOverride),
    ExtendHeatingUntilSleeping(ExtendHeatingUntilSleeping),
    DeferHeatingUntilVentilationDone(DeferHeatingUntilVentilationDone),
    ReduceNoiseAtNight(ReduceNoiseAtNight),
    SaveTvEnergy(SaveTvEnergy),
    FollowDefaultSetting(FollowDefaultSetting),
    UserTriggerAction(UserTriggerAction),
}

impl<API> Action<API> for HomeAction
where
    API: DataPointAccess<Powered>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<SetPoint>
        + DataPointAccess<RiskOfMould>
        + DataPointAccess<ColdAirComingIn>
        + DataPointAccess<Opened>
        + DataPointAccess<AutomaticTemperatureIncrease>
        + DataPointAccess<UserControlled>
        + DataPointAccess<RelativeHumidity>
        + DataPointAccess<Resident>
        + DataPointAccess<EnergySaving>
        + CommandState<Command>
        + CommandAccess<Command>
        + CommandAccess<SetHeating>
        + UserTriggerAccess,
{
    async fn evaluate(&self, api: &API) -> Result<ActionEvaluationResult> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.evaluate(api).await,
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window.evaluate(api).await
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open.evaluate(api).await
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation.evaluate(api).await
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => {
                no_heating_during_automatic_temperature_increase
                    .evaluate(api)
                    .await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.evaluate(api).await
            }
            HomeAction::KeepUserOverride(keep_user_override) => {
                keep_user_override.evaluate(api).await
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping.evaluate(api).await
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done.evaluate(api).await
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night.evaluate(api).await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => save_tv_energy.evaluate(api).await,
            HomeAction::FollowDefaultSetting(follow_default_setting) => {
                follow_default_setting.evaluate(&()).await
            }
            HomeAction::UserTriggerAction(user_trigger_action) => {
                user_trigger_action.evaluate(api).await
            }
        }
    }
}
