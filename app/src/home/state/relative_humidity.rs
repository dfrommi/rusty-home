use crate::core::time::DateTime;
use crate::core::unit::Percent;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
}

impl Estimatable for RelativeHumidity {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::linear(at, df)
    }
}
