pub use api::state::RelativeHumidity;
use support::{time::DateTime, unit::Percent, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for RelativeHumidity {
    type Type = Percent;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}
