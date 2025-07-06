use crate::core::time::DateTime;
use r#macro::Id;

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
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        algo::last_seen(at, df)
    }
}
