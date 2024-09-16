use crate::error::Result;

use super::*;
use crate::prelude::*;

use api::state::DataPoint;
pub use api::state::RelativeHumidity;
use support::unit::Percent;

impl DataPointAccess<Percent> for RelativeHumidity {
    fn current_data_point(&self) -> Result<DataPoint<Percent>> {
        Ok(home_api().get_latest(self)?)
    }
}

impl TimeSeriesAccess<Percent> for RelativeHumidity {
    fn series_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<TimeSeries<Percent>> {
        home_api().get_covering(self, since).map(TimeSeries::new)?
    }
}
