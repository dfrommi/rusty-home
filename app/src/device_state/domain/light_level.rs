use r#macro::{EnumVariants, Id};

use crate::core::unit::Lux;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum LightLevel {
    LivingRoom,
    Kitchen,
    Bedroom,
}

impl LightLevel {
    pub fn minimum_storage_delta(&self, _incoming_value: Lux) -> Option<Lux> {
        None
    }
}
