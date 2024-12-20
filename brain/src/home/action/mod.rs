mod dehumidify;
mod heating;
mod inform_window_open;
mod keep_user_override;
mod reduce_noise_at_night;
mod request_closing_window;
mod save_tv_energy;

use std::fmt::Debug;

use anyhow::Result;
use api::command::Command;

use api::command::PushNotify;
use api::command::SetEnergySaving;
use api::command::SetHeating;
use api::command::SetPower;
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

use crate::core::planner::{Action, ActionExecution};
use crate::home::state::*;
use crate::port::*;

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

//enum_dispatch is not able to implement for a given generic type-value
//TODO macro
impl<T> Action<T, Command> for HomeAction
where
    T: DataPointAccess<Powered>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<SetPoint>
        + DataPointAccess<RiskOfMould>
        + DataPointAccess<ColdAirComingIn>
        + DataPointAccess<Opened>
        + DataPointAccess<AutomaticTemperatureIncrease>
        + DataPointAccess<UserControlled>
        + DataPointAccess<RelativeHumidity>
        + DataPointAccess<Resident>
        + CommandAccess<Command>
        + CommandAccess<SetHeating>
        + CommandAccess<PushNotify>
        + CommandAccess<SetEnergySaving>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.preconditions_fulfilled(api).await,
            HomeAction::RequestClosingWindow(request_closing_window) => {
                request_closing_window.preconditions_fulfilled(api).await
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
            HomeAction::InformWindowOpen(inform_window_open) => {
                inform_window_open.preconditions_fulfilled(api).await
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                save_tv_energy.preconditions_fulfilled(api).await
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                ir_heater_auto_turn_off.preconditions_fulfilled(api).await
            }
        }
    }

    fn execution(&self) -> ActionExecution<Command> {
        match self {
            HomeAction::Dehumidify(dehumidify) => {
                <Dehumidify as Action<T, SetPower>>::execution(dehumidify).into()
            }
            HomeAction::RequestClosingWindow(request_closing_window) => {
                <RequestClosingWindow as Action<T, SetPower>>::execution(request_closing_window)
                    .into()
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                <NoHeatingDuringVentilation as Action<T, SetHeating>>::execution(
                    no_heating_during_ventilation,
                )
                .into()
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => <NoHeatingDuringAutomaticTemperatureIncrease as Action<T, SetHeating>>::execution(
                no_heating_during_automatic_temperature_increase,
            )
            .into(),
            HomeAction::KeepUserOverride(keep_user_override) => {
                <KeepUserOverride as Action<T, Command>>::execution(keep_user_override)
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                <ExtendHeatingUntilSleeping as Action<T, SetHeating>>::execution(
                    extend_heating_until_sleeping,
                )
                .into()
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                <DeferHeatingUntilVentilationDone as Action<T, SetHeating>>::execution(
                    defer_heating_until_ventilation_done,
                )
                .into()
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                <ReduceNoiseAtNight as Action<T, SetPower>>::execution(reduce_noise_at_night).into()
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                <InformWindowOpen as Action<T, PushNotify>>::execution(inform_window_open).into()
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                <SaveTvEnergy as Action<T, SetEnergySaving>>::execution(save_tv_energy).into()
            }
            HomeAction::IrHeaterAutoTurnOff(ir_heater_auto_turn_off) => {
                <IrHeaterAutoTurnOff as Action<T, SetPower>>::execution(ir_heater_auto_turn_off)
                    .into()
            }
        }
    }
}
