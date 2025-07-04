use r#macro::Id;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum TotalWaterConsumption {
    KitchenCold,
    KitchenWarm,
    BathroomCold,
    BathroomWarm,
}
