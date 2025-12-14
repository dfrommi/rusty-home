use r#macro::{EnumVariants, Id};

use crate::home::Thermostat;

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
            TotalRadiatorConsumption::LivingRoomBig => Thermostat::LivingRoomBig.heating_factor(),
            TotalRadiatorConsumption::LivingRoomSmall => Thermostat::LivingRoomSmall.heating_factor(),
            TotalRadiatorConsumption::Bedroom => Thermostat::Bedroom.heating_factor(),
            TotalRadiatorConsumption::Kitchen => Thermostat::Kitchen.heating_factor(),
            TotalRadiatorConsumption::RoomOfRequirements => Thermostat::RoomOfRequirements.heating_factor(),
            TotalRadiatorConsumption::Bathroom => Thermostat::Bathroom.heating_factor(),
        }
    }
}
