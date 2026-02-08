use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    LivingRoomBig,
    LivingRoomBigLower,
    LivingRoomSmall,
    LivingRoomSmallLower,
    Bedroom,
    BedroomLower,
    Kitchen,
    KitchenLower,
    RoomOfRequirements,
    RoomOfRequirementsLower,
    Bathroom,
    BathroomLower,
}
