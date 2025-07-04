use r#macro::Id;
use support::time::DateTime;

use crate::core::timeseries::interpolate::{Estimatable, algo};

//TODO impl anyoneSleeping. Requires impl of enum from crate

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    BedDennis,
    BedSabine,
    CouchLeft,
    CouchCenter,
    CouchRight,
}

impl Estimatable for Presence {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
