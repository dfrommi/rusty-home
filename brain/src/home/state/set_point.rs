pub use api::state::SetPoint;
use support::{time::DateTime, unit::DegreeCelsius};

use crate::core::timeseries::interpolate::{Estimatable, algo};

impl Estimatable for SetPoint {
    type Type = DegreeCelsius;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
