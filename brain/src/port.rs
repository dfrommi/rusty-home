#![allow(async_fn_in_trait)]

use anyhow::Result;
use support::{
    DataPoint, ValueObject, t,
    time::{DateTime, DateTimeRange},
};

use crate::core::timeseries::{TimeSeries, interpolate::Estimatable};

pub trait DataPointAccess<T: ValueObject> {
    async fn current_data_point(&self, item: T) -> Result<DataPoint<T::ValueType>>;

    async fn current(&self, item: T) -> Result<T::ValueType> {
        self.current_data_point(item).await.map(|dp| dp.value)
    }
}

pub trait TimeSeriesAccess<T>
where
    T: Estimatable,
{
    async fn series(&self, item: T, range: DateTimeRange) -> Result<TimeSeries<T>>;

    async fn series_since(&self, item: T, since: DateTime) -> Result<TimeSeries<T>> {
        self.series(item, DateTimeRange::new(since, t!(now))).await
    }
}

pub enum CommandExecutionResult {
    Triggered,
    Skipped,
}
