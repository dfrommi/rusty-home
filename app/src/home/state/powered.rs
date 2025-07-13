use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Powered {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}
