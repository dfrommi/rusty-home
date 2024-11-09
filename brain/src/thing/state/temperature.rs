use super::*;
use crate::{adapter::persistence::StateRepository, prelude::*};
use anyhow::Result;
use support::unit::DegreeCelsius;

pub use api::state::Temperature;

impl DataPointAccess<DegreeCelsius> for Temperature {
    async fn current_data_point(&self) -> Result<DataPoint<DegreeCelsius>> {
        home_api().get_latest(self).await
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
            .map(|v| TimeSeries::new(v, since))?
    }
}
