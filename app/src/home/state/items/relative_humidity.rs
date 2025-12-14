use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    BedroomOuterWall,
    Kitchen,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
    LivingRoomTado,
    RoomOfRequirementsTado,
    BedroomTado,
}
