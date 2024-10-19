mod dehumidify;
mod heating;
mod request_closing_window;

use std::fmt::Debug;

use anyhow::Result;
use api::{command::Thermostat, state::SetPoint};
use enum_dispatch::enum_dispatch;

pub use dehumidify::Dehumidify;
pub use heating::Heat;
pub use heating::NoHeatingDuringAutomaticTemperatureIncrease;
pub use heating::NoHeatingDuringVentilation;
pub use request_closing_window::RequestClosingWindow;

use crate::thing::UserControlled;

#[derive(Debug, Clone)]
#[enum_dispatch(Action)]
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
pub trait Action: Debug {
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

    pub fn user_controlled(&self) -> UserControlled {
        match self {
            HeatingZone::LivingRoom => UserControlled::LivingRoomThermostat,
            HeatingZone::Bedroom => UserControlled::BedroomThermostat,
            HeatingZone::Kitchen => UserControlled::KitchenThermostat,
            HeatingZone::RoomOfRequirements => UserControlled::RoomOfRequirementsThermostat,
            HeatingZone::Bathroom => UserControlled::BathroomThermostat,
        }
    }
}
