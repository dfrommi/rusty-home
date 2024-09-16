use crate::home_api;
use api::state::DataPoint;
pub use api::state::TotalEnergyConsumption;
use support::unit::KiloWattHours;

use super::*;
use crate::prelude::*;

use super::DataPointAccess;

impl DataPointAccess<KiloWattHours> for TotalEnergyConsumption {
    async fn current_data_point(&self) -> Result<DataPoint<KiloWattHours>> {
        Ok(home_api().get_latest(self).await?)
    }
}
