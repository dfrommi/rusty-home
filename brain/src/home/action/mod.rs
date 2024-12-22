mod dehumidify;
mod heating;
mod inform_window_open;
mod keep_user_override;
mod reduce_noise_at_night;
mod request_closing_window;
mod save_tv_energy;

use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Result;
use api::command::Command;

use api::command::CommandSource;
use api::command::CommandTarget;
use api::command::SetHeating;
use api::state::ExternalAutoControl;
use api::state::Powered;
use api::state::RelativeHumidity;
use api::state::SetPoint;
pub use dehumidify::Dehumidify;
pub use heating::*;
pub use inform_window_open::InformWindowOpen;
pub use keep_user_override::KeepUserOverride;
pub use reduce_noise_at_night::ReduceNoiseAtNight;
pub use request_closing_window::RequestClosingWindow;
pub use save_tv_energy::SaveTvEnergy;

use crate::core::planner::CommandState;
use crate::core::planner::ConditionalAction;
use crate::core::planner::ExecutableAction;
use crate::core::planner::ExecutionAwareAction;
use crate::core::planner::Lockable;
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
}

impl<API> ConditionalAction<API> for HomeAction
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
        + CommandState<Command>
        + CommandAccess<Command>
        + CommandAccess<SetHeating>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.preconditions_fulfilled(api).await,
            HomeAction::RequestClosingWindow(closing_window) => {
                closing_window.preconditions_fulfilled(api).await
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open.preconditions_fulfilled(api).await
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation
                    .preconditions_fulfilled(api)
                    .await
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => {
                no_heating_during_automatic_temperature_increase
                    .preconditions_fulfilled(api)
                    .await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.preconditions_fulfilled(api).await
            }
            HomeAction::KeepUserOverride(keep_user_override) => {
                keep_user_override.preconditions_fulfilled(api).await
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping
                    .preconditions_fulfilled(api)
                    .await
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done
                    .preconditions_fulfilled(api)
                    .await
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night.preconditions_fulfilled(api).await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                save_tv_energy.preconditions_fulfilled(api).await
            }
        }
    }
}

impl<E> ExecutableAction<E> for HomeAction
where
    E: CommandExecutor<Command> + CommandState<Command> + CommandAccess<Command>,
{
    async fn execute(&self, executor: &E) -> Result<CommandExecutionResult> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.execute(executor).await,
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window.execute(executor).await
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open.execute(executor).await
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation.execute(executor).await
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => {
                no_heating_during_automatic_temperature_increase
                    .execute(executor)
                    .await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.execute(executor).await
            }
            HomeAction::KeepUserOverride(keep_user_override) => {
                keep_user_override.execute(&()).await
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping.execute(executor).await
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done.execute(executor).await
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night.execute(executor).await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => save_tv_energy.execute(executor).await,
        }
    }
}

impl<API> ExecutionAwareAction<API> for HomeAction
where
    API: CommandState<Command> + CommandAccess<Command>,
{
    async fn was_latest_execution_for_target_since(
        &self,
        since: support::time::DateTime,
        api: &API,
    ) -> Result<bool> {
        match self {
            HomeAction::Dehumidify(dehumidify) => {
                dehumidify
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => {
                no_heating_during_automatic_temperature_increase
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::KeepUserOverride(keep_user_override) => {
                keep_user_override
                    .was_latest_execution_for_target_since(since, &())
                    .await
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                save_tv_energy
                    .was_latest_execution_for_target_since(since, api)
                    .await
            }
        }
    }

    async fn is_reflected_in_state(&self, api: &API) -> Result<bool> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.is_reflected_in_state(api).await,
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window.is_reflected_in_state(api).await
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation
                    .is_reflected_in_state(api)
                    .await
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => {
                no_heating_during_automatic_temperature_increase
                    .is_reflected_in_state(api)
                    .await
            }
            HomeAction::KeepUserOverride(keep_user_override) => {
                keep_user_override.is_reflected_in_state(&()).await
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping
                    .is_reflected_in_state(api)
                    .await
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done
                    .is_reflected_in_state(api)
                    .await
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night.is_reflected_in_state(api).await
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open.is_reflected_in_state(api).await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                save_tv_energy.is_reflected_in_state(api).await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.is_reflected_in_state(api).await
            }
        }
    }
}

impl Lockable<CommandTarget> for HomeAction {
    fn locking_key(&self) -> CommandTarget {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.locking_key(),
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window.locking_key()
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                no_heating_during_ventilation.locking_key()
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => no_heating_during_automatic_temperature_increase.locking_key(),
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.locking_key()
            }
            HomeAction::KeepUserOverride(keep_user_override) => keep_user_override.locking_key(),
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                extend_heating_until_sleeping.locking_key()
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                defer_heating_until_ventilation_done.locking_key()
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                reduce_noise_at_night.locking_key()
            }
            HomeAction::InformWindowOpen(inform_window_open) => inform_window_open.locking_key(),
            HomeAction::SaveTvEnergy(save_tv_energy) => save_tv_energy.locking_key(),
        }
    }
}
