pub use api::state::Presence;
use support::{time::DateTime, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for Presence {
    type Type = bool;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::last_seen(at, prev, next)
    }
}
