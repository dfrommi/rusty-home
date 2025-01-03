pub use api::state::RelativeHumidity;
use support::{time::DateTime, unit::Percent};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for RelativeHumidity {
    type Type = Percent;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::linear(at, df)
    }
}
