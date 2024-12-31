pub use api::state::HeatingDemand;
use support::{time::DateTime, unit::Percent, DataFrame};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for HeatingDemand {
    type Type = Percent;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
