use r#macro::Id;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}
