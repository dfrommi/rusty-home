pub use api::state::SetPoint;
use support::{time::DateTime, unit::DegreeCelsius, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for SetPoint {
    type Type = DegreeCelsius;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::last_seen(at, prev, next)
    }
}
