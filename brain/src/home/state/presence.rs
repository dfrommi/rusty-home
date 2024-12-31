pub use api::state::Presence;
use support::time::DateTime;

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for Presence {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
