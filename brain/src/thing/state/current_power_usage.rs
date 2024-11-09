use crate::{
    adapter::persistence::{DataPoint, StateRepository},
    home_api,
};
pub use api::state::CurrentPowerUsage;
use support::unit::Watt;

use anyhow::Result;

use super::DataPointAccess;

impl DataPointAccess<Watt> for CurrentPowerUsage {
    async fn current_data_point(&self) -> Result<DataPoint<Watt>> {
        home_api().get_latest(self).await
    }
}
