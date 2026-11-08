use crate::core::domain::Radiator;
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
    pub fn radiator(&self) -> Radiator {
        match self {
            HeatingDemand::LivingRoomBig => Radiator::LivingRoomBig,
            HeatingDemand::LivingRoomSmall => Radiator::LivingRoomSmall,
            HeatingDemand::Bedroom => Radiator::Bedroom,
            HeatingDemand::Kitchen => Radiator::Kitchen,
            HeatingDemand::RoomOfRequirements => Radiator::RoomOfRequirements,
            HeatingDemand::Bathroom => Radiator::Bathroom,
        }
    }

    pub fn scaling_factor(&self) -> f64 {
        self.radiator().heating_factor()
    }
}
