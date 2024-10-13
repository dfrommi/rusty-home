use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;
use anyhow::Result;

use api::state::value_type::PowerState;
pub use api::state::Powered;

impl DataPointAccess<bool> for Powered {
    async fn current_data_point(&self) -> Result<DataPoint<bool>> {
        let dp = home_api().get_latest(self).await?;
        Ok(dp.map_value(|v| *v == PowerState::On))
    }
}
