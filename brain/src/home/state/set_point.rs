use r#macro::Id;
use support::{time::DateTime, unit::DegreeCelsius};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum SetPoint {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for SetPoint {
    type Type = DegreeCelsius;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
