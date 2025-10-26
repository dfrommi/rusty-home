#![allow(async_fn_in_trait)]

use crate::core::HomeApi;
use crate::core::timeseries::DataFrame;
use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
};
use anyhow::Result;

use crate::core::timeseries::{DataPoint, TimeSeries, interpolate::Estimatable};
use crate::home::state::HomeStateValueType;

pub trait DataPointAccess<T: HomeStateValueType> {
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<T::ValueType>>;

    async fn current(&self, api: &HomeApi) -> Result<T::ValueType> {
        self.current_data_point(api).await.map(|dp| dp.value)
    }
}

pub trait DataFrameAccess<T: HomeStateValueType> {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<T::ValueType>>;
}

pub trait TimeSeriesAccess<T: Estimatable> {
    async fn series(&self, range: DateTimeRange, api: &HomeApi) -> Result<TimeSeries<T>>;

    async fn series_since(&self, since: DateTime, api: &HomeApi) -> Result<TimeSeries<T>> {
        self.series(DateTimeRange::new(since, t!(now)), api).await
    }
}

impl<T> TimeSeriesAccess<T> for T
where
    T: DataFrameAccess<T> + Estimatable + Clone,
{
    async fn series(&self, range: DateTimeRange, api: &HomeApi) -> Result<TimeSeries<T>> {
        let df = self.get_data_frame(range.clone(), api).await?;
        TimeSeries::new(self.clone(), &df, range)
    }
}
