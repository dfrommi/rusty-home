use crate::error::Result;

use super::*;
use crate::prelude::*;

use api::state::DataPoint;
pub use api::state::RelativeHumidity;
use support::unit::Percent;

impl DataPointAccess<Percent> for RelativeHumidity {
    async fn current_data_point(&self) -> Result<DataPoint<Percent>> {
        Ok(home_api().get_latest(self).await?)
    }
}

impl TimeSeriesAccess<Percent> for RelativeHumidity {
    async fn series_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<TimeSeries<Percent>> {
        home_api()
            .get_covering(self, since)
            .await
            .map(TimeSeries::new)?
    }
}
