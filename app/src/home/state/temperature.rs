use crate::core::time::DateTime;
use crate::core::unit::DegreeCelsius;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
}

impl Estimatable for Temperature {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        algo::linear(at, df)
    }
}
