use r#macro::Id;
use crate::core::time::DateTime;

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

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

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}
