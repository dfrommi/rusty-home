use crate::error::Result;

use super::*;
use crate::prelude::*;

use api::state::DataPoint;
pub use api::state::Temperature;
use support::unit::DegreeCelsius;

impl DataPointAccess<DegreeCelsius> for Temperature {
    fn current_data_point(&self) -> Result<DataPoint<DegreeCelsius>> {
        Ok(home_api().get_latest(self)?)
    }
}

impl TimeSeriesAccess<DegreeCelsius> for Temperature {
    fn series_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<TimeSeries<DegreeCelsius>> {
        home_api().get_covering(self, since).map(TimeSeries::new)?
    }
}
