use std::{fmt::Debug, marker::PhantomData};

mod current_power_usage;
mod dewpoint;
mod powered;
mod relative_humidity;
mod risk_of_mould;
mod temperature;
mod total_energy_consumption;
mod user_controlled;

use api::state::DataPoint;
pub use current_power_usage::CurrentPowerUsage;
pub use dewpoint::DewPoint;
pub use powered::Powered;
pub use relative_humidity::RelativeHumidity;
pub use risk_of_mould::RiskOfMould;
pub use temperature::Temperature;
pub use total_energy_consumption::TotalEnergyConsumption;
pub use user_controlled::UserControlled;

use crate::prelude::*;
use crate::support::timeseries::TimeSeries;

pub trait DataPointAccess<T> {
    fn current_data_point(&self) -> Result<DataPoint<T>>;

    fn current(&self) -> Result<T> {
        self.current_data_point().map(|dp| dp.value)
    }
}

pub trait TimeSeriesAccess<T> {
    fn series_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<TimeSeries<T>>;

    fn series_of_last(&self, duration: ::chrono::Duration) -> Result<TimeSeries<T>> {
        self.series_since(chrono::Utc::now() - duration)
    }
}

pub struct MultiTimeSeriesAccess<
    S: From<f64> + Into<f64>,
    T: From<f64> + Into<f64>,
    R: From<f64> + Into<f64>,
    U: TimeSeriesAccess<S>,
    V: TimeSeriesAccess<T>,
    F: Fn(&S, &T) -> R,
> {
    first: U,
    second: V,
    merge: F,

    _marker_s: PhantomData<S>,
    _marker_t: PhantomData<T>,
}

impl<
        S: From<f64> + Into<f64>,
        T: From<f64> + Into<f64>,
        R: From<f64> + Into<f64>,
        U: TimeSeriesAccess<S>,
        V: TimeSeriesAccess<T>,
        F: Fn(&S, &T) -> R,
    > MultiTimeSeriesAccess<S, T, R, U, V, F>
{
    pub fn new(first: U, second: V, merge: F) -> Self {
        Self {
            first,
            second,
            merge,

            _marker_s: PhantomData,
            _marker_t: PhantomData,
        }
    }
}

impl<
        S: From<f64> + Into<f64> + Debug,
        T: From<f64> + Into<f64> + Debug,
        R: From<f64> + Into<f64> + Debug,
        U: TimeSeriesAccess<S>,
        V: TimeSeriesAccess<T>,
        F: Fn(&S, &T) -> R,
    > TimeSeriesAccess<R> for MultiTimeSeriesAccess<S, T, R, U, V, F>
{
    fn series_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<TimeSeries<R>> {
        let first_series = self.first.series_since(since)?;
        let second_series = self.second.series_since(since)?;

        println!("First {:?}", first_series);
        println!("Second {:?}", second_series);

        let mut dps: Vec<DataPoint<R>> = Vec::new();

        for first_dp in first_series.iter() {
            if let Some(second_dp) = second_series.at_or_latest_before(first_dp.timestamp) {
                let value = (self.merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        for second_dp in second_series.iter() {
            if let Some(first_dp) = first_series.at_or_latest_before(second_dp.timestamp) {
                let value = (self.merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        let ts = TimeSeries::new(dps);
        println!("Final {:?}", ts);
        ts
    }
}
