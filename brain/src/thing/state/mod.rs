mod automatic_temp_inc;
mod cold_air_coming_in;
//mod current_power_usage;
mod dewpoint;
//mod external_auto_control;
mod opened;
//mod powered;
//mod relative_humidity;
mod resident;
mod risk_of_mould;
//mod set_point;
//mod temperature;
//mod total_energy_consumption;
mod user_controlled;

use api::state::ChannelTypeInfo;
pub use api::state::Powered;
pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use opened::Opened;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
use support::time::DateTime;
pub use user_controlled::UserControlled;

use crate::adapter::persistence::DataPoint;
use crate::support::timeseries::interpolate::Interpolatable;
use crate::support::timeseries::TimeSeries;
use anyhow::Result;

pub trait DataPointAccess<T: ChannelTypeInfo> {
    async fn current_data_point(&self, item: T) -> Result<DataPoint<T::ValueType>>;

    async fn current(&self, item: T) -> Result<T::ValueType> {
        self.current_data_point(item).await.map(|dp| dp.value)
    }
}

pub trait TimeSeriesAccess<T>
where
    T: ChannelTypeInfo,
    T::ValueType: Clone + Interpolatable,
{
    async fn series_since(&self, item: T, since: DateTime) -> Result<TimeSeries<T::ValueType>>;
}
