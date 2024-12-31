pub use api::state::Temperature;
use support::{time::DateTime, unit::DegreeCelsius};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for Temperature {
    type Type = DegreeCelsius;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::linear(at, df)
    }
}
