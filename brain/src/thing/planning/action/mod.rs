mod dehumidify;
mod heating;
mod request_closing_window;

use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Result;
use api::state::ExternalAutoControl;
use api::state::Temperature;
use api::{command::Thermostat, state::SetPoint};
use enum_dispatch::enum_dispatch;

pub use dehumidify::Dehumidify;
pub use heating::Heat;
pub use heating::NoHeatingDuringAutomaticTemperatureIncrease;
pub use heating::NoHeatingDuringVentilation;
pub use request_closing_window::RequestClosingWindow;

use crate::thing::UserControlled;

#[derive(Debug, Clone)]
#[enum_dispatch(Action, Dislay)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    Heat(Heat),
    NoHeatingDuringVentilation(NoHeatingDuringVentilation),
    NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resource {
    Dehumidifier,
    LivingRoomNotificationLight,
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
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
    async fn preconditions_fulfilled(&self) -> Result<bool>;
    async fn is_running(&self) -> Result<bool>;
    async fn is_user_controlled(&self) -> Result<bool>;

    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;

    fn controls_resource(&self) -> Option<Resource>;
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

    pub fn resource(&self) -> Resource {
        match self {
            HeatingZone::LivingRoom => Resource::LivingRoomThermostat,
            HeatingZone::Bedroom => Resource::BedroomThermostat,
            HeatingZone::Kitchen => Resource::KitchenThermostat,
            HeatingZone::RoomOfRequirements => Resource::RoomOfRequirementsThermostat,
            HeatingZone::Bathroom => Resource::BathroomThermostat,
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

    pub fn current_room_temperature(&self) -> Temperature {
        match self {
            HeatingZone::LivingRoom => Temperature::LivingRoomDoor,
            HeatingZone::Bedroom => Temperature::BedroomDoor,
            HeatingZone::Kitchen => Temperature::KitchenOuterWall,
            HeatingZone::RoomOfRequirements => Temperature::RoomOfRequirementsDoor,
            HeatingZone::Bathroom => Temperature::BathroomShower,
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
            HomeAction::Heat(heat) => write!(f, "{}", heat),
            HomeAction::NoHeatingDuringVentilation(no_heating_during_ventilation) => {
                write!(f, "{}", no_heating_during_ventilation)
            }
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(
                no_heating_during_automatic_temperature_increase,
            ) => write!(f, "{}", no_heating_during_automatic_temperature_increase),
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
