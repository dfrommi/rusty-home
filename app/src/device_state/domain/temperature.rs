use crate::home::Thermostat;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    BedroomOuterWall,
    Kitchen,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
    ThermostatOnDevice(Thermostat),
    ThermostatExternal(Thermostat),
    LivingRoomTado,
    RoomOfRequirementsTado,
    BedroomTado,
}
