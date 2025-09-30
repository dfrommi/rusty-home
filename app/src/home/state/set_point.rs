use crate::core::time::DateTime;
use crate::core::unit::DegreeCelsius;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for SetPoint {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        algo::last_seen(at, df)
    }
}
