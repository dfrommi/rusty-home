use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::{
    core::unit::DegreeCelsius,
    home::{
        command::HeatingTargetState,
        state::{HeatingDemand, HeatingMode, SetPoint},
    },
};

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

#[derive(Debug, Clone, Hash, Eq, PartialEq, derive_more::Display, Id, EnumVariants)]
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
            HeatingZone::Bedroom => DegreeCelsius(19.0),
            HeatingZone::Kitchen => DegreeCelsius(17.0),
            HeatingZone::RoomOfRequirements => DegreeCelsius(18.0),
            HeatingZone::Bathroom => DegreeCelsius(15.0),
        }
    }

    pub fn heating_state(&self, mode: &HeatingMode) -> HeatingTargetState {
        let default_temperature = self.default_setpoint();

        match mode {
            HeatingMode::Ventilation | HeatingMode::PostVentilation => HeatingTargetState::WindowOpen,
            HeatingMode::EnergySaving => HeatingTargetState::Heat {
                temperature: default_temperature,
            },
            HeatingMode::Comfort => HeatingTargetState::Heat {
                temperature: default_temperature + DegreeCelsius(1.0),
            },
            HeatingMode::Sleep => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(1.0),
            },
            HeatingMode::Away => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(2.0),
            },
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
