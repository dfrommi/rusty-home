mod dehumidify;
mod heating;
mod keep_user_override;
mod request_closing_window;

use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Result;
use api::command::Command;
use api::command::CommandTarget;
use api::state::ExternalAutoControl;
use api::{command::Thermostat, state::SetPoint};
use enum_dispatch::enum_dispatch;

pub use dehumidify::Dehumidify;
pub use heating::*;
pub use keep_user_override::KeepUserOverride;
pub use request_closing_window::RequestClosingWindow;

#[derive(Debug, Clone)]
#[enum_dispatch(Action, Dislay)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    NoHeatingDuringVentilation(NoHeatingDuringVentilation),
    NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease),
    KeepUserOverride(KeepUserOverride),
    ExtendHeatingUntilSleeping(ExtendHeatingUntilSleeping),
    DeferHeatingUntilVentilationDone(DeferHeatingUntilVentilationDone),
}

#[derive(Debug, Clone)]
pub enum HeatingZone {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[enum_dispatch]
pub trait Action: Debug + Display {
    //action should be started based on current state
    async fn preconditions_fulfilled(&self) -> Result<bool>;

    //action was just triggered or effect of action is fulfilled based on current state
    async fn is_running(&self) -> Result<bool>;

    fn start_command(&self) -> Option<Command>;

    fn stop_command(&self) -> Option<Command>;

    fn controls_target(&self) -> Option<CommandTarget> {
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

                Some(start)
            }
            (Some(start), None) => Some(start),
            (None, Some(stop)) => Some(stop),
            (None, None) => None,
        }
    }
}

impl HeatingZone {
    pub fn thermostat(&self) -> Thermostat {
        match self {
            HeatingZone::LivingRoom => Thermostat::LivingRoom,
            HeatingZone::Bedroom => Thermostat::Bedroom,
            HeatingZone::Kitchen => Thermostat::Kitchen,
            HeatingZone::RoomOfRequirements => Thermostat::RoomOfRequirements,
            HeatingZone::Bathroom => Thermostat::Bathroom,
        }
    }

    pub fn current_set_point(&self) -> SetPoint {
        match self {
            HeatingZone::LivingRoom => SetPoint::LivingRoom,
            HeatingZone::Bedroom => SetPoint::Bedroom,
            HeatingZone::Kitchen => SetPoint::Kitchen,
            HeatingZone::RoomOfRequirements => SetPoint::RoomOfRequirements,
            HeatingZone::Bathroom => SetPoint::Bathroom,
        }
    }

    pub fn auto_mode(&self) -> ExternalAutoControl {
        match self {
            HeatingZone::LivingRoom => ExternalAutoControl::LivingRoomThermostat,
            HeatingZone::Bedroom => ExternalAutoControl::BedroomThermostat,
            HeatingZone::Kitchen => ExternalAutoControl::KitchenThermostat,
            HeatingZone::RoomOfRequirements => ExternalAutoControl::RoomOfRequirementsThermostat,
            HeatingZone::Bathroom => ExternalAutoControl::BathroomThermostat,
        }
    }
}

impl Display for HomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HomeAction::Dehumidify(dehumidify) => write!(f, "{}", dehumidify),
            HomeAction::RequestClosingWindow(request_closing_window) => {
                write!(f, "{}", request_closing_window)
            }
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                write!(f, "{}", no_heating_during_ventilation)
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => write!(f, "{}", no_heating_during_automatic_temperature_increase),
            HomeAction::KeepUserOverride(keep_user_override) => {
                write!(f, "{}", keep_user_override)
            }
            HomeAction::ExtendHeatingUntilSleeping(extend_heating_until_sleeping) => {
                write!(f, "{}", extend_heating_until_sleeping)
            }
            HomeAction::DeferHeatingUntilVentilationDone(defer_heating_until_ventilation_done) => {
                write!(f, "{}", defer_heating_until_ventilation_done)
            }
        }
    }
}

impl Display for HeatingZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeatingZone::LivingRoom => write!(f, "LivingRoom"),
            HeatingZone::Bedroom => write!(f, "Bedroom"),
            HeatingZone::Kitchen => write!(f, "Kitchen"),
            HeatingZone::RoomOfRequirements => write!(f, "RoomOfRequirements"),
            HeatingZone::Bathroom => write!(f, "Bathroom"),
        }
    }
}
