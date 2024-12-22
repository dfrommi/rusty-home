#![allow(async_fn_in_trait)]

use anyhow::Result;
use api::{
    command::{Command, CommandExecution, CommandSource, CommandTarget},
    state::ChannelTypeInfo,
};
use support::{
    t,
    time::{DateTime, DateTimeRange},
    DataPoint,
};

use crate::{
    core::planner::ActionResult,
    support::timeseries::{interpolate::Estimatable, TimeSeries},
};

pub trait DataPointAccess<T: ChannelTypeInfo> {
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

pub trait CommandAccess<C: Into<Command>> {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution<C>>>;

    async fn get_all_commands(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution<C>>>;

    async fn get_latest_command_source(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandSource>>;
}

pub enum CommandExecutionResult {
    Triggered,
    Skipped,
}
pub trait CommandExecutor<C: Into<Command>> {
    async fn execute(&self, command: C, source: CommandSource) -> Result<CommandExecutionResult>;
}

pub trait PlanningResultTracer {
    async fn add_planning_trace(&self, results: &[ActionResult]) -> anyhow::Result<()>;
}
