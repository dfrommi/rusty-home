use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum ExternalAutoControl {
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}
