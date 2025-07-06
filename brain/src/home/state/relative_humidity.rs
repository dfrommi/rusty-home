use r#macro::{EnumVariants, Id};
use support::{time::DateTime, unit::Percent};

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
    type Type = Percent;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::linear(at, df)
    }
}
