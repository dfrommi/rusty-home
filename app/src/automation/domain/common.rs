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
    pub fn radiators(&self) -> Vec<Radiator> {
        match self {
            HeatingZone::LivingRoom => vec![Radiator::LivingRoomBig, Radiator::LivingRoomSmall],
            HeatingZone::Bedroom => vec![Radiator::Bedroom],
            HeatingZone::Kitchen => vec![Radiator::Kitchen],
            HeatingZone::RoomOfRequirements => vec![Radiator::RoomOfRequirements],
            HeatingZone::Bathroom => vec![Radiator::Bathroom],
        }
    }

    pub fn inside_temperature(&self) -> Temperature {
        Temperature::HeatingZone(*self)
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

    pub fn set_point(&self) -> SetPoint {
        match self {
            Radiator::LivingRoomBig => SetPoint::Radiator(Radiator::LivingRoomBig),
            Radiator::LivingRoomSmall => SetPoint::Radiator(Radiator::LivingRoomSmall),
            Radiator::Bedroom => SetPoint::Radiator(Radiator::Bedroom),
            Radiator::Kitchen => SetPoint::Radiator(Radiator::Kitchen),
            Radiator::RoomOfRequirements => SetPoint::Radiator(Radiator::RoomOfRequirements),
            Radiator::Bathroom => SetPoint::Radiator(Radiator::Bathroom),
        }
    }

    pub fn surface_temperature(&self) -> Temperature {
        Temperature::Radiator(*self)
    }

    pub fn room_temperature(&self) -> Temperature {
        self.heating_zone().inside_temperature()
    }

    pub fn heating_demand(&self) -> HeatingDemand {
        match self {
            Radiator::LivingRoomBig => HeatingDemand::Radiator(Radiator::LivingRoomBig),
            Radiator::LivingRoomSmall => HeatingDemand::Radiator(Radiator::LivingRoomSmall),
            Radiator::Bedroom => HeatingDemand::Radiator(Radiator::Bedroom),
            Radiator::Kitchen => HeatingDemand::Radiator(Radiator::Kitchen),
            Radiator::RoomOfRequirements => HeatingDemand::Radiator(Radiator::RoomOfRequirements),
            Radiator::Bathroom => HeatingDemand::Radiator(Radiator::Bathroom),
        }
    }
}
