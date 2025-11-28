use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::{
    core::unit::DegreeCelsius,
    home::state::{HeatingDemand, HeatingMode, OpenedArea, SetPoint, Temperature},
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Thermostat {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancedThermostat {
    LivingRoomBig,
    LivingRoomSmall,
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

    pub fn default_setpoint(&self) -> DegreeCelsius {
        match self {
            HeatingZone::LivingRoom => DegreeCelsius(19.0),
            HeatingZone::Bedroom => DegreeCelsius(18.5),
            HeatingZone::Kitchen => DegreeCelsius(17.0),
            HeatingZone::RoomOfRequirements => DegreeCelsius(18.0),
            HeatingZone::Bathroom => DegreeCelsius(15.0),
        }
    }

    pub fn setpoint_for_mode(&self, mode: &HeatingMode) -> DegreeCelsius {
        let default_temperature = self.default_setpoint();

        match mode {
            HeatingMode::Manual(t, _) => *t,
            HeatingMode::Ventilation => DegreeCelsius(0.0),
            HeatingMode::PostVentilation => default_temperature,
            HeatingMode::EnergySaving => default_temperature,
            HeatingMode::Sleep if self == &HeatingZone::LivingRoom => default_temperature - DegreeCelsius(0.5),
            HeatingMode::Comfort if self == &HeatingZone::LivingRoom => default_temperature + DegreeCelsius(0.5),
            HeatingMode::Sleep if self == &HeatingZone::Bedroom => default_temperature - DegreeCelsius(0.5),
            HeatingMode::Sleep => default_temperature - DegreeCelsius(1.0),
            HeatingMode::Comfort => default_temperature + DegreeCelsius(1.0),
            HeatingMode::Away => default_temperature - DegreeCelsius(2.0),
        }
    }

    //TODO use in actions
    pub fn inside_temperature(&self) -> Temperature {
        match self {
            HeatingZone::LivingRoom => Temperature::LivingRoomTado,
            HeatingZone::Bedroom => Temperature::BedroomTado,
            HeatingZone::Kitchen => Temperature::Kitchen,
            HeatingZone::RoomOfRequirements => Temperature::RoomOfRequirementsTado,
            HeatingZone::Bathroom => Temperature::BathroomShower,
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

impl From<&LoadBalancedThermostat> for Thermostat {
    fn from(value: &LoadBalancedThermostat) -> Self {
        match value {
            LoadBalancedThermostat::LivingRoomBig => Thermostat::LivingRoomBig,
            LoadBalancedThermostat::LivingRoomSmall => Thermostat::LivingRoomSmall,
        }
    }
}
