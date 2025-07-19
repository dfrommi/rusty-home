use r#macro::{EnumVariants, Id};

use crate::core::{
    time::DateTime,
    timeseries::{
        DataFrame,
        interpolate::{self, Estimatable},
    },
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Powered {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}

impl Estimatable for Powered {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        interpolate::algo::last_seen(at, df)
    }
}
