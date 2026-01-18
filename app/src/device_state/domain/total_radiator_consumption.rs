use r#macro::{EnumVariants, Id};

use crate::automation::Radiator;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl TotalRadiatorConsumption {
    pub fn scaling_factor(&self) -> f64 {
        match self {
            TotalRadiatorConsumption::LivingRoomBig => Radiator::LivingRoomBig.heating_factor(),
            TotalRadiatorConsumption::LivingRoomSmall => Radiator::LivingRoomSmall.heating_factor(),
            TotalRadiatorConsumption::Bedroom => Radiator::Bedroom.heating_factor(),
            TotalRadiatorConsumption::Kitchen => Radiator::Kitchen.heating_factor(),
            TotalRadiatorConsumption::RoomOfRequirements => Radiator::RoomOfRequirements.heating_factor(),
            TotalRadiatorConsumption::Bathroom => Radiator::Bathroom.heating_factor(),
        }
    }
}
