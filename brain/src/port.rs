#![allow(async_fn_in_trait)]

use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{Command, CommandExecution, CommandId, CommandSource, CommandTarget},
    state::ChannelTypeInfo,
};
use support::time::DateTime;

use crate::{
    adapter::persistence::DataPoint,
    support::timeseries::{interpolate::Interpolatable, TimeSeries},
    thing::planning::ActionResult,
};

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

pub trait CommandAccess<C: CommandId> {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution<C::CommandType>>>;

    async fn get_all_commands(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution<C::CommandType>>>;

    async fn get_latest_command_source(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandSource>>;
}

pub trait CommandExecutor<C: Into<Command>> {
    async fn execute(&self, command: C, source: CommandSource) -> Result<()>;
}

pub trait PlanningResultTracer {
    async fn add_planning_trace<'a, A: Display>(
        &self,
        results: &[ActionResult<'a, A>],
    ) -> anyhow::Result<()>;
}
