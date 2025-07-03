pub use api::state::HeatingDemand;
use support::{DataFrame, time::DateTime, unit::Percent};

use crate::core::timeseries::interpolate::{Estimatable, algo};

impl Estimatable for HeatingDemand {
    type Type = Percent;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
