use r#macro::{EnumVariants, Id};

use crate::core::{
    time::DateTime,
    timeseries::{
        DataFrame,
        interpolate::{self, Estimatable},
    },
};

use super::HeatingUnit;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for TotalRadiatorConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<HeatingUnit>) -> Option<HeatingUnit> {
        interpolate::algo::linear(at, df)
    }
}
