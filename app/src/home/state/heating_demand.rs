use crate::core::time::DateTime;
use crate::core::unit::Percent;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for HeatingDemand {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::last_seen(at, df)
    }
}
