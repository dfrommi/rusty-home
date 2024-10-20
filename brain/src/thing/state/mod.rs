mod automatic_temp_inc;
mod cold_air_coming_in;
mod current_power_usage;
mod dewpoint;
mod external_auto_control;
mod opened;
mod powered;
mod relative_humidity;
mod risk_of_mould;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod user_controlled;

pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use powered::Powered;
pub use risk_of_mould::RiskOfMould;
pub use set_point::SetPoint;
pub use temperature::Temperature;
pub use user_controlled::UserControlled;

use crate::adapter::persistence::DataPoint;
use crate::support::timeseries::TimeSeries;
use anyhow::Result;

pub trait DataPointAccess<T> {
    async fn current_data_point(&self) -> Result<DataPoint<T>>;

    async fn current(&self) -> Result<T> {
        self.current_data_point().await.map(|dp| dp.value)
    }
}

pub trait TimeSeriesAccess<T> {
    async fn series_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<TimeSeries<T>>;

    async fn series_of_last(&self, duration: ::chrono::Duration) -> Result<TimeSeries<T>> {
        self.series_since(chrono::Utc::now() - duration).await
    }
}
