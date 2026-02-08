use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemandLimit {
    LivingRoomBigUpper,
    LivingRoomBigLower,
    LivingRoomSmallUpper,
    LivingRoomSmallLower,
    BedroomUpper,
    BedroomLower,
    KitchenUpper,
    KitchenLower,
    RoomOfRequirementsUpper,
    RoomOfRequirementsLower,
    BathroomUpper,
    BathroomLower,
}
