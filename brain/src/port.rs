#![allow(async_fn_in_trait)]

use anyhow::Result;
use api::{
    command::{Command, CommandExecution, CommandSource, CommandTarget},
    state::ChannelValue,
    trigger::{UserTrigger, UserTriggerTarget},
};
use support::{
    t,
    time::{DateTime, DateTimeRange},
    DataPoint, ValueObject,
};

use crate::{
    core::planner::PlanningTrace,
    support::timeseries::{interpolate::Estimatable, TimeSeries},
};

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

pub trait DataPointStore {
    async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<ChannelValue>>>;
}

pub trait CommandAccess {
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

    async fn get_all_commands(
        &self,
        from: DateTime,
        until: DateTime,
    ) -> Result<Vec<CommandExecution>>;
}

pub enum CommandExecutionResult {
    Triggered,
    Skipped,
}

pub trait CommandExecutor {
    async fn execute(
        &self,
        command: Command,
        source: CommandSource,
    ) -> Result<CommandExecutionResult>;
}

pub trait PlanningResultTracer {
    async fn add_planning_trace(&self, results: &PlanningTrace) -> anyhow::Result<()>;

    async fn get_latest_planning_trace(&self, before: DateTime) -> anyhow::Result<PlanningTrace>;

    async fn get_planning_traces_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<PlanningTrace>>;

    async fn get_planning_traces_by_trace_id(
        &self,
        trace_id: &str,
    ) -> anyhow::Result<Option<PlanningTrace>>;

    async fn get_trace_ids(&self, range: DateTimeRange) -> anyhow::Result<Vec<(String, DateTime)>>;
}

pub trait CommandStore {
    async fn save_command(
        &self,
        command: Command,
        source: CommandSource,
        correlation_id: Option<String>,
    ) -> anyhow::Result<()>;
}

pub trait UserTriggerExecutor {
    async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()>;
}

pub trait UserTriggerAccess {
    async fn latest_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<DataPoint<UserTrigger>>>;
}
