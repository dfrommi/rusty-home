use crate::error::Result;

use super::*;
use crate::prelude::*;

use api::state::DataPoint;
pub use api::state::Temperature;
use support::unit::DegreeCelsius;

impl DataPointAccess<DegreeCelsius> for Temperature {
    async fn current_data_point(&self) -> Result<DataPoint<DegreeCelsius>> {
        Ok(home_api().get_latest(self).await?)
    }
}

impl TimeSeriesAccess<DegreeCelsius> for Temperature {
    async fn series_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<TimeSeries<DegreeCelsius>> {
        home_api()
            .get_covering(self, since)
            .await
            .map(TimeSeries::new)?
    }
}
