use crate::home::Thermostat;
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
            HeatingDemand::LivingRoomBig => Thermostat::LivingRoomBig.heating_factor(),
            HeatingDemand::LivingRoomSmall => Thermostat::LivingRoomSmall.heating_factor(),
            HeatingDemand::Bedroom => Thermostat::Bedroom.heating_factor(),
            HeatingDemand::Kitchen => Thermostat::Kitchen.heating_factor(),
            HeatingDemand::RoomOfRequirements => Thermostat::RoomOfRequirements.heating_factor(),
            HeatingDemand::Bathroom => Thermostat::Bathroom.heating_factor(),
        }
    }
}
