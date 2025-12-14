use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalWaterConsumption {
    KitchenCold,
    KitchenWarm,
    BathroomCold,
    BathroomWarm,
}
