use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemandLimit {
    LivingRoomBigUpper,
    LivingRoomSmallUpper,
    BedroomUpper,
    KitchenUpper,
    RoomOfRequirementsUpper,
    BathroomUpper,
}
