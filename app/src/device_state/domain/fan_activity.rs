use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum FanActivity {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
    BedroomDehumidifier,
}
