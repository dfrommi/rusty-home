use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::home_state::{HeatingDemand, Temperature};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Id, EnumVariants, derive_more::Display)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Id, EnumVariants, derive_more::Display)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum RoomWithWindow {
    #[display("LivingRoom")]
    LivingRoom,
    #[display("Bedroom")]
    Bedroom,
    #[display("Kitchen")]
    Kitchen,
    #[display("RoomOfRequirements")]
    RoomOfRequirements,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, derive_more::Display, Id, EnumVariants, Serialize, Deserialize)]
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
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl HeatingZone {
    pub fn room(&self) -> Room {
        match self {
            HeatingZone::LivingRoom => Room::LivingRoom,
            HeatingZone::Bedroom => Room::Bedroom,
            HeatingZone::Kitchen => Room::Kitchen,
            HeatingZone::RoomOfRequirements => Room::RoomOfRequirements,
            HeatingZone::Bathroom => Room::Bathroom,
        }
    }

    pub fn radiators(&self) -> Vec<Radiator> {
        match self {
            HeatingZone::LivingRoom => vec![Radiator::LivingRoomBig, Radiator::LivingRoomSmall],
            HeatingZone::Bedroom => vec![Radiator::Bedroom],
            HeatingZone::Kitchen => vec![Radiator::Kitchen],
            HeatingZone::RoomOfRequirements => vec![Radiator::RoomOfRequirements],
            HeatingZone::Bathroom => vec![Radiator::Bathroom],
        }
    }

    pub fn room_temperature(&self) -> Temperature {
        Temperature::Room(self.room())
    }
}

impl Radiator {
    pub fn heating_factor(&self) -> f64 {
        match self {
            Radiator::LivingRoomBig => 1.728,
            Radiator::LivingRoomSmall => 0.501,
            Radiator::Bedroom => 1.401,
            Radiator::Kitchen => 1.485,
            Radiator::RoomOfRequirements => 1.193,
            Radiator::Bathroom => 0.496,
        }
    }

    pub fn heating_zone(&self) -> HeatingZone {
        match self {
            Radiator::LivingRoomBig | Radiator::LivingRoomSmall => HeatingZone::LivingRoom,
            Radiator::Bedroom => HeatingZone::Bedroom,
            Radiator::Kitchen => HeatingZone::Kitchen,
            Radiator::RoomOfRequirements => HeatingZone::RoomOfRequirements,
            Radiator::Bathroom => HeatingZone::Bathroom,
        }
    }

    pub fn surface_temperature(&self) -> Temperature {
        Temperature::Radiator(*self)
    }

    pub fn room_temperature(&self) -> Temperature {
        self.heating_zone().room_temperature()
    }

    pub fn current_heating_demand(&self) -> HeatingDemand {
        HeatingDemand::Radiator(*self)
    }
}
