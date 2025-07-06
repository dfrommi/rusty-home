use r#macro::Id;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum ExternalAutoControl {
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}
