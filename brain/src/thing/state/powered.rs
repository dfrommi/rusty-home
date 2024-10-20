use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;
use anyhow::Result;

pub use api::state::Powered;

impl DataPointAccess<bool> for Powered {
    async fn current_data_point(&self) -> Result<DataPoint<bool>> {
        home_api().get_latest(self).await
    }
}
