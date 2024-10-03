use crate::{adapter::persistence::DataPoint, home_api};
pub use api::state::CurrentPowerUsage;
use support::unit::Watt;

use crate::prelude::*;

use super::DataPointAccess;

impl DataPointAccess<Watt> for CurrentPowerUsage {
    async fn current_data_point(&self) -> Result<DataPoint<Watt>> {
        home_api().get_latest(self).await
    }
}
