use r#macro::{EnumVariants, Id};

use crate::core::domain::Radiator;

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
    pub fn radiator(&self) -> Radiator {
        match self {
            TotalRadiatorConsumption::LivingRoomBig => Radiator::LivingRoomBig,
            TotalRadiatorConsumption::LivingRoomSmall => Radiator::LivingRoomSmall,
            TotalRadiatorConsumption::Bedroom => Radiator::Bedroom,
            TotalRadiatorConsumption::Kitchen => Radiator::Kitchen,
            TotalRadiatorConsumption::RoomOfRequirements => Radiator::RoomOfRequirements,
            TotalRadiatorConsumption::Bathroom => Radiator::Bathroom,
        }
    }

    pub fn scaling_factor(&self) -> f64 {
        self.radiator().heating_factor()
    }
}
