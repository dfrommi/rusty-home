use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    CouchLeft,
    CouchCenter,
    CouchRight,
    KitchenArea,
    LivingRoomArea,
    LivingRoomCouch,
    BedroomBed,
}
