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

use api::command::EnergySavingDevice;
use api::command::NotificationTarget;
use api::command::Thermostat;
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

use crate::port::*;
use crate::state::*;


#[derive(Debug, Clone, derive_more::Display, derive_more::From)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    InformWindowOpen(InformWindowOpen),
    NoHeatingDuringVentilation(NoHeatingDuringVentilation),
    NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease),
    KeepUserOverride(KeepUserOverride),
    ExtendHeatingUntilSleeping(ExtendHeatingUntilSleeping),
    DeferHeatingUntilVentilationDone(DeferHeatingUntilVentilationDone),
    ReduceNoiseAtNight(ReduceNoiseAtNight),
    SaveTvEnergy(SaveTvEnergy),
}

pub trait Action<T>: Display {
    //action should be started based on current state
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool>;

    fn start_command(&self) -> Option<Command>;

    fn start_command_source(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:start", self))
    }

    fn stop_command(&self) -> Option<Command>;

    fn stop_command_source(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:stop", self))
    }

    fn controls_target(&self) -> CommandTarget {
        let start_target = self.start_command().map(|c| CommandTarget::from(&c));
        let stop_target = self.stop_command().map(|c| CommandTarget::from(&c));

        match (start_target, stop_target) {
            (Some(start), Some(stop)) => {
                if start != stop {
                    tracing::error!(
                        "Action {} controls different devices in start and stop commands. Falling back to start command",
                        self
                    );
                }

                start
            }
            (Some(start), None) => start,
            (None, Some(stop)) => stop,
            (None, None) => panic!("Action {} has no start or stop command and relies on default implementation of controls_target", self),
        }
    }
}

//enum_dispatch is not able to implement for a given generic type-value
//TODO macro
impl<T> Action<T> for HomeAction
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
        + CommandAccess<Thermostat>
        + CommandAccess<NotificationTarget>
        + CommandAccess<EnergySavingDevice>,
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
        }
    }

    fn start_command(&self) -> Option<Command> {
        match self {
            HomeAction::Dehumidify(dehumidify) => {
                <Dehumidify as Action<T>>::start_command(dehumidify)
            }
            HomeAction::RequestClosingWindow(request_closing_window) => {
                <RequestClosingWindow as Action<T>>::start_command(request_closing_window)
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                <NoHeatingDuringVentilation as Action<T>>::start_command(
                    no_heating_during_ventilation,
                )
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => <NoHeatingDuringAutomaticTemperatureIncrease as Action<T>>::start_command(
                no_heating_during_automatic_temperature_increase,
            ),
            HomeAction::KeepUserOverride(keep_user_override) => {
                <KeepUserOverride as Action<T>>::start_command(keep_user_override)
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                <ExtendHeatingUntilSleeping as Action<T>>::start_command(
                    extend_heating_until_sleeping,
                )
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                <DeferHeatingUntilVentilationDone as Action<T>>::start_command(
                    defer_heating_until_ventilation_done,
                )
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                <ReduceNoiseAtNight as Action<T>>::start_command(reduce_noise_at_night)
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                <InformWindowOpen as Action<T>>::start_command(inform_window_open)
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                <SaveTvEnergy as Action<T>>::start_command(save_tv_energy)
            }
        }
    }

    fn stop_command(&self) -> Option<Command> {
        match self {
            HomeAction::Dehumidify(dehumidify) => {
                <Dehumidify as Action<T>>::stop_command(dehumidify)
            }
            HomeAction::RequestClosingWindow(request_closing_window) => {
                <RequestClosingWindow as Action<T>>::stop_command(request_closing_window)
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                <NoHeatingDuringVentilation as Action<T>>::stop_command(
                    no_heating_during_ventilation,
                )
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => <NoHeatingDuringAutomaticTemperatureIncrease as Action<T>>::stop_command(
                no_heating_during_automatic_temperature_increase,
            ),
            HomeAction::KeepUserOverride(keep_user_override) => {
                <KeepUserOverride as Action<T>>::stop_command(keep_user_override)
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                <ExtendHeatingUntilSleeping as Action<T>>::stop_command(
                    extend_heating_until_sleeping,
                )
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                <DeferHeatingUntilVentilationDone as Action<T>>::stop_command(
                    defer_heating_until_ventilation_done,
                )
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                <ReduceNoiseAtNight as Action<T>>::stop_command(reduce_noise_at_night)
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                <InformWindowOpen as Action<T>>::stop_command(inform_window_open)
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                <SaveTvEnergy as Action<T>>::stop_command(save_tv_energy)
            }
        }
    }

    fn controls_target(&self) -> CommandTarget {
        match self {
            HomeAction::Dehumidify(dehumidify) => {
                <Dehumidify as Action<T>>::controls_target(dehumidify)
            }
            HomeAction::RequestClosingWindow(request_closing_window) => {
                <RequestClosingWindow as Action<T>>::controls_target(request_closing_window)
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                <NoHeatingDuringVentilation as Action<T>>::controls_target(
                    no_heating_during_ventilation,
                )
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => <NoHeatingDuringAutomaticTemperatureIncrease as Action<T>>::controls_target(
                no_heating_during_automatic_temperature_increase,
            ),
            HomeAction::KeepUserOverride(keep_user_override) => {
                <KeepUserOverride as Action<T>>::controls_target(keep_user_override)
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                <ExtendHeatingUntilSleeping as Action<T>>::controls_target(
                    extend_heating_until_sleeping,
                )
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                <DeferHeatingUntilVentilationDone as Action<T>>::controls_target(
                    defer_heating_until_ventilation_done,
                )
            }
            HomeAction::ReduceNoiseAtNight(reduce_noise_at_night) => {
                <ReduceNoiseAtNight as Action<T>>::controls_target(reduce_noise_at_night)
            }
            HomeAction::InformWindowOpen(inform_window_open) => {
                <InformWindowOpen as Action<T>>::controls_target(inform_window_open)
            }
            HomeAction::SaveTvEnergy(save_tv_energy) => {
                <SaveTvEnergy as Action<T>>::controls_target(save_tv_energy)
            }
        }
    }
}
