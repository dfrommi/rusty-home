use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Opened {
    KitchenWindow,
    BedroomWindow,
    LivingRoomWindowLeft,
    LivingRoomWindowRight,
    LivingRoomWindowSide,
    LivingRoomBalconyDoor,
    RoomOfRequirementsWindowLeft,
    RoomOfRequirementsWindowRight,
    RoomOfRequirementsWindowSide,
}
