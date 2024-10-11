use anyhow::Result;

use super::*;
use crate::prelude::*;

pub use api::state::RelativeHumidity;
use support::unit::Percent;

impl DataPointAccess<Percent> for RelativeHumidity {
    async fn current_data_point(&self) -> Result<DataPoint<Percent>> {
        home_api().get_latest(self).await
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
            .map(|v| TimeSeries::new(v, since))?
    }
}
