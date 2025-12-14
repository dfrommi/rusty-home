use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Opened {
    KitchenWindow,
    KitchenRadiatorThermostat,
    BedroomWindow,
    BedroomRadiatorThermostat,
    LivingRoomWindowLeft,
    LivingRoomWindowRight,
    LivingRoomWindowSide,
    LivingRoomBalconyDoor,
    LivingRoomRadiatorThermostatSmall,
    LivingRoomRadiatorThermostatBig,
    RoomOfRequirementsWindowLeft,
    RoomOfRequirementsWindowRight,
    RoomOfRequirementsWindowSide,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}
