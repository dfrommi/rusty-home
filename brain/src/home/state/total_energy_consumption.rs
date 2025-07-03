pub use api::state::TotalEnergyConsumption;
use support::{time::DateTime, unit::KiloWattHours};

use crate::core::timeseries::interpolate::{Estimatable, algo};

impl Estimatable for TotalEnergyConsumption {
    type Type = KiloWattHours;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::linear(at, df)
    }
}
