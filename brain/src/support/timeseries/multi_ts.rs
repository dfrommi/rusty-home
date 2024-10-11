use crate::{adapter::persistence::DataPoint, prelude::TimeSeriesAccess};
use std::{fmt::Debug, marker::PhantomData};

use super::TimeSeries;

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
        S: From<f64> + Into<f64> + Clone + Debug,
        T: From<f64> + Into<f64> + Clone + Debug,
        R: From<f64> + Into<f64> + Clone + Debug,
        U: TimeSeriesAccess<S>,
        V: TimeSeriesAccess<T>,
        F: Fn(&S, &T) -> R,
    > TimeSeriesAccess<R> for MultiTimeSeriesAccess<S, T, R, U, V, F>
{
    async fn series_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<TimeSeries<R>> {
        let first_series = self.first.series_since(since).await?;
        let second_series = self.second.series_since(since).await?;

        let mut dps: Vec<DataPoint<R>> = Vec::new();

        for first_dp in first_series.iter() {
            if let Some(second_dp) = second_series.at(first_dp.timestamp) {
                let value = (self.merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        for second_dp in second_series.iter() {
            if let Some(first_dp) = first_series.at(second_dp.timestamp) {
                let value = (self.merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        let ts = TimeSeries::new(dps, since);
        println!("Final {:?}", ts);
        ts
    }
}
