use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::{
    core::unit::DegreeCelsius,
    home_state::{HeatingDemand, HeatingMode, OpenedArea, SetPoint, Temperature},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Id, EnumVariants, derive_more::Display)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum Room {
    #[display("LivingRoom")]
    LivingRoom,
    #[display("Bedroom")]
    Bedroom,
    #[display("Kitchen")]
    Kitchen,
    #[display("RoomOfRequirements")]
    RoomOfRequirements,
    #[display("Bathroom")]
    Bathroom,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, derive_more::Display, Id, EnumVariants, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeatingZone {
    #[display("LivingRoom")]
    LivingRoom,
    #[display("Bedroom")]
    Bedroom,
    #[display("Kitchen")]
    Kitchen,
    #[display("RoomOfRequirements")]
    RoomOfRequirements,
    #[display("Bathroom")]
    Bathroom,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Thermostat {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl HeatingZone {
    pub fn thermostats(&self) -> Vec<Thermostat> {
        match self {
            HeatingZone::LivingRoom => vec![Thermostat::LivingRoomBig, Thermostat::LivingRoomSmall],
            HeatingZone::Bedroom => vec![Thermostat::Bedroom],
            HeatingZone::Kitchen => vec![Thermostat::Kitchen],
            HeatingZone::RoomOfRequirements => vec![Thermostat::RoomOfRequirements],
            HeatingZone::Bathroom => vec![Thermostat::Bathroom],
        }
    }

    pub fn for_thermostat(thermostat: &Thermostat) -> HeatingZone {
        match thermostat {
            Thermostat::LivingRoomBig | Thermostat::LivingRoomSmall => HeatingZone::LivingRoom,
            Thermostat::Bedroom => HeatingZone::Bedroom,
            Thermostat::Kitchen => HeatingZone::Kitchen,
            Thermostat::RoomOfRequirements => HeatingZone::RoomOfRequirements,
            Thermostat::Bathroom => HeatingZone::Bathroom,
        }
    }

    //TODO use in actions
    pub fn inside_temperature(&self) -> Temperature {
        match self {
            HeatingZone::LivingRoom => Temperature::LivingRoom,
            HeatingZone::Bedroom => Temperature::Bedroom,
            HeatingZone::Kitchen => Temperature::Kitchen,
            HeatingZone::RoomOfRequirements => Temperature::RoomOfRequirements,
            HeatingZone::Bathroom => Temperature::Bathroom,
        }
    }

    pub fn window(&self) -> OpenedArea {
        match self {
            HeatingZone::LivingRoom => OpenedArea::LivingRoomWindowOrDoor,
            HeatingZone::Kitchen => OpenedArea::KitchenWindow,
            HeatingZone::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow,
            HeatingZone::Bedroom | HeatingZone::Bathroom => OpenedArea::BedroomWindow,
        }
    }
}

impl Thermostat {
    pub fn heating_factor(&self) -> f64 {
        match self {
            Thermostat::LivingRoomBig => 1.728,
            Thermostat::LivingRoomSmall => 0.501,
            Thermostat::Bedroom => 1.401,
            Thermostat::Kitchen => 1.485,
            Thermostat::RoomOfRequirements => 1.193,
            Thermostat::Bathroom => 0.496,
        }
    }

    pub fn set_point(&self) -> SetPoint {
        match self {
            Thermostat::LivingRoomBig => SetPoint::LivingRoomBig,
            Thermostat::LivingRoomSmall => SetPoint::LivingRoomSmall,
            Thermostat::Bedroom => SetPoint::Bedroom,
            Thermostat::Kitchen => SetPoint::Kitchen,
            Thermostat::RoomOfRequirements => SetPoint::RoomOfRequirements,
            Thermostat::Bathroom => SetPoint::Bathroom,
        }
    }

    pub fn surface_temperature(&self) -> Temperature {
        Temperature::Radiator(*self)
    }

    pub fn heating_demand(&self) -> HeatingDemand {
        match self {
            Thermostat::LivingRoomBig => HeatingDemand::LivingRoomBig,
            Thermostat::LivingRoomSmall => HeatingDemand::LivingRoomSmall,
            Thermostat::Bedroom => HeatingDemand::Bedroom,
            Thermostat::Kitchen => HeatingDemand::Kitchen,
            Thermostat::RoomOfRequirements => HeatingDemand::RoomOfRequirements,
            Thermostat::Bathroom => HeatingDemand::Bathroom,
        }
    }
}
