#![allow(async_fn_in_trait)]

use crate::core::{HomeApi, ValueObject};
use crate::home::command::{CommandExecution, CommandTarget};
use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
};
use anyhow::Result;

use crate::core::timeseries::{DataPoint, TimeSeries, interpolate::Estimatable};

pub trait DataPointAccess<T: ValueObject> {
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<T::ValueType>>;

    async fn current(&self, api: &HomeApi) -> Result<T::ValueType> {
        self.current_data_point(api).await.map(|dp| dp.value)
    }
}

pub trait TimeSeriesAccess<T: Estimatable> {
    async fn series(&self, range: DateTimeRange, api: &HomeApi) -> Result<TimeSeries<T>>;

    async fn series_since(&self, since: DateTime, api: &HomeApi) -> Result<TimeSeries<T>> {
        self.series(DateTimeRange::new(since, t!(now)), api).await
    }
}

pub trait CommandExecutionAccess {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>>;

    async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>>;
}

pub enum CommandExecutionResult {
    Triggered,
    Skipped,
}
