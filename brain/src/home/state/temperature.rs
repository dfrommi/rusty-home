pub use api::state::Temperature;
use support::{time::DateTime, unit::DegreeCelsius, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for Temperature {
    type Type = DegreeCelsius;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}
