use r#macro::{EnumVariants, Id};
use support::{DataFrame, time::DateTime, unit::Percent};

use crate::core::timeseries::interpolate::{Estimatable, algo};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for HeatingDemand {
    type Type = Percent;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
