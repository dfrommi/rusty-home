use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;
use anyhow::Result;

pub use api::state::Powered;
use support::unit::PowerState;

impl DataPointAccess<PowerState> for Powered {
    async fn current_data_point(&self) -> Result<DataPoint<PowerState>> {
        home_api().get_latest(self).await
    }
}
