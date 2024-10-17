use super::*;
use crate::prelude::*;
use anyhow::Result;
use support::unit::DegreeCelsius;

pub use api::state::SetPoint;

impl DataPointAccess<DegreeCelsius> for SetPoint {
    async fn current_data_point(&self) -> Result<DataPoint<DegreeCelsius>> {
        home_api().get_latest(self).await
    }
}
