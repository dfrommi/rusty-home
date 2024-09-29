use crate::home_api;
pub use api::state::CurrentPowerUsage;
use api::state::DataPoint;
use support::unit::Watt;

use crate::prelude::*;

use super::DataPointAccess;

impl DataPointAccess<Watt> for CurrentPowerUsage {
    async fn current_data_point(&self) -> Result<DataPoint<Watt>> {
        Ok(home_api().get_latest(self).await?)
    }
}
