use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum PowerAvailable {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}
