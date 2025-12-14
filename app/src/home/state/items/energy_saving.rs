use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergySaving {
    LivingRoomTv,
}
