pub use api::state::TotalEnergyConsumption;
use support::{time::DateTime, unit::KiloWattHours, DataPoint};

use crate::support::timeseries::interpolate::{algo, Estimatable};

impl Estimatable for TotalEnergyConsumption {
    type Type = KiloWattHours;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}
