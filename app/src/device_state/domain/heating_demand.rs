use crate::automation::Radiator;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl HeatingDemand {
    pub fn scaling_factor(&self) -> f64 {
        match self {
            HeatingDemand::LivingRoomBig => Radiator::LivingRoomBig.heating_factor(),
            HeatingDemand::LivingRoomSmall => Radiator::LivingRoomSmall.heating_factor(),
            HeatingDemand::Bedroom => Radiator::Bedroom.heating_factor(),
            HeatingDemand::Kitchen => Radiator::Kitchen.heating_factor(),
            HeatingDemand::RoomOfRequirements => Radiator::RoomOfRequirements.heating_factor(),
            HeatingDemand::Bathroom => Radiator::Bathroom.heating_factor(),
        }
    }
}
