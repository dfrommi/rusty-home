pub use api::state::Presence;
use support::time::DateTime;

use crate::core::timeseries::interpolate::{Estimatable, algo};

//TODO impl anyoneSleeping. Requires impl of enum from crate

impl Estimatable for Presence {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
