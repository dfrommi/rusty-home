use crate::{adapter::persistence::DataPoint, home_api};
pub use api::state::TotalEnergyConsumption;
use support::unit::KiloWattHours;

use crate::prelude::*;

use super::DataPointAccess;

impl DataPointAccess<KiloWattHours> for TotalEnergyConsumption {
    async fn current_data_point(&self) -> Result<DataPoint<KiloWattHours>> {
        home_api().get_latest(self).await
    }
}
