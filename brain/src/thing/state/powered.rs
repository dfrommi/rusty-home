use crate::{adapter::persistence::DataPoint, error::Result, home_api};

use super::DataPointAccess;

pub use api::state::Powered;
use support::unit::PowerState;

impl DataPointAccess<PowerState> for Powered {
    async fn current_data_point(&self) -> Result<DataPoint<PowerState>> {
        home_api().get_latest(self).await
    }
}
