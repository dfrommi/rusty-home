use super::persistence::{Database, OfflineItem};
use super::time::{DateTime, DateTimeRange};
use super::timeseries::{DataPoint, TimeSeries, interpolate::Estimatable};
use super::planner::PlanningTrace;
use super::ValueObject;
use crate::home::command::{Command, CommandExecution, CommandTarget};
use crate::home::state::{PersistentHomeState, PersistentHomeStateValue};
use crate::home::trigger::{UserTrigger, UserTriggerTarget};
use crate::port::{CommandExecutionAccess, DataPointAccess, TimeSeriesAccess, CommandExecutionResult};
use crate::home::command::CommandSource;
use crate::core::ItemAvailability;
use anyhow::Result;
use std::fmt::Debug;

#[derive(Clone)]
pub struct HomeApi {
    db: Database,
}

impl HomeApi {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // State Management Methods
    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        self.db.preload_ts_cache().await
    }

    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        self.db.invalidate_ts_cache(tag_id).await
    }

    pub async fn add_state(&self, value: &PersistentHomeStateValue, timestamp: &DateTime) -> Result<()> {
        self.db.add_state(value, timestamp).await
    }

    // Command Management Methods
    pub async fn execute(&self, command: Command, source: CommandSource) -> anyhow::Result<CommandExecutionResult> {
        self.db.execute(command, source).await
    }

    pub async fn get_command_for_processing(&self) -> Result<Option<CommandExecution>> {
        self.db.get_command_for_processing().await
    }

    pub async fn set_command_state_success(&self, command_id: i64) -> Result<()> {
        self.db.set_command_state_success(command_id).await
    }

    pub async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> Result<()> {
        self.db.set_command_state_error(command_id, error_message).await
    }

    pub async fn invalidate_command_cache(&self, target: &CommandTarget) {
        self.db.invalidate_command_cache(target).await
    }

    // User Trigger Methods
    pub async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        self.db.add_user_trigger(trigger).await
    }

    pub async fn latest_since(&self, target: &UserTriggerTarget, since: DateTime) -> anyhow::Result<Option<DataPoint<UserTrigger>>> {
        self.db.latest_since(target, since).await
    }

    // Planning Trace Methods
    pub async fn add_planning_trace(&self, result: &PlanningTrace) -> anyhow::Result<()> {
        self.db.add_planning_trace(result).await
    }

    // Availability Methods
    pub async fn add_item_availability(&self, item: ItemAvailability) -> anyhow::Result<()> {
        self.db.add_item_availability(item).await
    }
}

// Grafana-specific methods (read-only queries for dashboards)
impl HomeApi {
    pub async fn get_all_data_points_in_range(&self, range: DateTimeRange) -> anyhow::Result<Vec<DataPoint<PersistentHomeStateValue>>> {
        self.db.get_all_data_points_in_range(range).await
    }

    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> Result<Vec<CommandExecution>> {
        self.db.get_all_commands(from, until).await
    }

    pub async fn get_latest_planning_trace(&self, before: DateTime) -> anyhow::Result<PlanningTrace> {
        self.db.get_latest_planning_trace(before).await
    }

    pub async fn get_planning_traces_by_trace_id(&self, trace_id: &str) -> anyhow::Result<Option<PlanningTrace>> {
        self.db.get_planning_traces_by_trace_id(trace_id).await
    }

    pub async fn get_trace_ids(&self, range: DateTimeRange) -> anyhow::Result<Vec<(String, DateTime)>> {
        self.db.get_trace_ids(range).await
    }

    pub async fn get_planning_traces_in_range(&self, range: DateTimeRange) -> anyhow::Result<Vec<PlanningTrace>> {
        self.db.get_planning_traces_in_range(range).await
    }

    pub async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        self.db.get_offline_items().await
    }
}

// Trait Implementations
impl<T> DataPointAccess<T> for HomeApi
where
    T: Into<PersistentHomeState> + ValueObject + Debug + Clone,
{
    async fn current_data_point(&self, item: T) -> Result<DataPoint<T::ValueType>> {
        self.db.current_data_point(item).await
    }
}

impl<T> TimeSeriesAccess<T> for HomeApi
where
    T: Into<PersistentHomeState> + Estimatable + Clone + Debug,
{
    async fn series(&self, item: T, range: DateTimeRange) -> Result<TimeSeries<T>> {
        self.db.series(item, range).await
    }
}

impl CommandExecutionAccess for HomeApi {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>> {
        self.db.get_latest_command(target, since).await
    }

    async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        self.db.get_all_commands_for_target(target, since).await
    }
}
