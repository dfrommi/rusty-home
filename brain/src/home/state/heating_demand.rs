pub use api::state::HeatingDemand;
use support::{time::DateTime, unit::Percent, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for HeatingDemand {
    type Type = Percent;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::last_seen(at, prev, next)
    }
}
