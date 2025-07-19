use r#macro::{EnumVariants, Id};

use crate::core::{
    time::DateTime,
    timeseries::{
        DataFrame,
        interpolate::{self, Estimatable},
    },
};

use super::KiloCubicMeter;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalWaterConsumption {
    KitchenCold,
    KitchenWarm,
    BathroomCold,
    BathroomWarm,
}

impl Estimatable for TotalWaterConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<KiloCubicMeter>) -> Option<KiloCubicMeter> {
        interpolate::algo::linear(at, df)
    }
}
