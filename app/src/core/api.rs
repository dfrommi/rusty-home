use super::ValueObject;
use super::persistence::{Database, OfflineItem};
use super::planner::PlanningTrace;
use super::time::{DateTime, DateTimeRange, Duration};
use super::timeseries::{DataFrame, DataPoint, TimeSeries, interpolate::Estimatable};
use crate::core::ItemAvailability;
use crate::home::command::{Command, CommandExecution, CommandTarget};
use crate::home::state::{HomeState, PersistentHomeState, PersistentHomeStateValue};
use crate::home::trigger::{UserTrigger, UserTriggerTarget};
use crate::port::{CommandExecutionAccess, CommandExecutionResult, DataFrameAccess, DataPointAccess, TimeSeriesAccess};
use crate::t;
use anyhow::Result;
use infrastructure::TraceContext;
use r#macro::mockable;
use moka::future::Cache;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct HomeApi {
    db: Database,
    caching_range: CachingRange,
    ts_cache: Cache<i64, Arc<(DateTimeRange, DataFrame<f64>)>>,
    cmd_cache: Cache<CommandTarget, Arc<(DateTimeRange, Vec<CommandExecution>)>>,
    #[cfg(test)]
    state_dp_mock: std::collections::HashMap<HomeState, DataPoint<f64>>,
    #[cfg(test)]
    state_ts_mock: std::collections::HashMap<HomeState, DataFrame<f64>>,
}

#[derive(Debug, Clone)]
pub enum CachingRange {
    OfLast(Duration),
    Fixed(DateTime, DateTime),
}

impl Default for CachingRange {
    fn default() -> Self {
        CachingRange::OfLast(t!(72 hours))
    }
}

impl HomeApi {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            caching_range: CachingRange::default(),
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(3 * 60 * 60))
                .build(),
            cmd_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(3 * 60 * 60))
                .build(),
            #[cfg(test)]
            state_dp_mock: std::collections::HashMap::new(),
            #[cfg(test)]
            state_ts_mock: std::collections::HashMap::new(),
        }
    }
}

//
// CACHING
//
impl HomeApi {
    // Cache Management Methods
    fn caching_range(&self) -> DateTimeRange {
        match &self.caching_range {
            CachingRange::OfLast(duration) => DateTimeRange::new(t!(now) - duration.clone(), DateTime::max_value()),
            CachingRange::Fixed(start, end) => DateTimeRange::new(*start, *end),
        }
    }

    fn is_readonly_cache(&self) -> bool {
        DateTime::is_shifted()
    }

    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        tracing::debug!("Start preloading cache");

        let tag_ids = self.db.get_all_tag_ids().await?;

        for tag_id in tag_ids {
            self.get_dataframe_from_cache(tag_id, &self.caching_range()).await;
        }

        tracing::debug!("Preloading cache done");
        Ok(())
    }

    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        tracing::debug!("Invalidating timeseries cache for tag {}", tag_id);
        self.ts_cache.invalidate(&tag_id).await;
    }

    pub async fn invalidate_command_cache(&self, target: &CommandTarget) {
        tracing::debug!("Invalidating command cache for target {:?}", target);
        self.cmd_cache.invalidate(target).await;
    }

    //try to return reference or at least avoid copy of entire dataframe
    async fn get_dataframe_from_cache(
        &self,
        tag_id: i64,
        range: &DateTimeRange,
    ) -> Option<Arc<(DateTimeRange, DataFrame<f64>)>> {
        let cached = if self.is_readonly_cache() {
            Ok(self.ts_cache.get(&tag_id))
        } else {
            let df = self
                .ts_cache
                .try_get_with(tag_id, async {
                    tracing::debug!("No cached data found for tag {}, fetching from database", tag_id);
                    let cache_range = self.caching_range();
                    self.db
                        .get_dataframe_for_tag(tag_id, &cache_range)
                        .await
                        .map(|df| Arc::new((cache_range, df)))
                })
                .await;
            Some(df).transpose()
        };

        match cached {
            Ok(Some(cached)) if cached.0.covers(range) => Some(cached),
            Err(e) => {
                tracing::error!("Error fetching dataframe for tag {} from cache or init cacke: {:?}", tag_id, e);
                None
            }
            _ => None,
        }
    }

    async fn get_commands_from_cache(
        &self,
        target: &CommandTarget,
        range: &DateTimeRange,
    ) -> Option<Arc<(DateTimeRange, Vec<CommandExecution>)>> {
        let cached = if self.is_readonly_cache() {
            Ok(self.cmd_cache.get(target))
        } else {
            let commands = self
                .cmd_cache
                .try_get_with(target.clone(), async {
                    tracing::debug!("No command-cache entry found for target {:?}", target);
                    let cache_range = self.caching_range();

                    self.db
                        .query_all_commands(Some(target.clone()), &cache_range)
                        .await
                        .map(|cmds| Arc::new((cache_range, cmds)))
                })
                .await;
            Some(commands).transpose()
        };

        match cached {
            Ok(Some(cached)) if cached.0.covers(range) => Some(cached),
            Err(e) => {
                tracing::error!(
                    "Error fetching commands for target {:?} from cache or init cache: {:?}",
                    target,
                    e
                );
                None
            }
            _ => None,
        }
    }
}

//
//STATE
//
impl HomeApi {
    async fn get_datapoint(&self, tag_id: i64, at: &DateTime) -> Result<DataPoint<f64>> {
        let range = DateTimeRange::new(*at - t!(2 minutes), *at);
        match self.get_dataframe(tag_id, &range).await?.prev_or_at(*at) {
            Some(dp) => Ok(dp.clone()),
            None => anyhow::bail!("No data point found for tag {} at {}", tag_id, at),
        }
    }

    async fn get_dataframe(&self, tag_id: i64, range: &DateTimeRange) -> Result<DataFrame<f64>> {
        match self.get_dataframe_from_cache(tag_id, range).await {
            Some(df) => df.1.retain_range_with_context(range),
            None => {
                tracing::warn!("No cached data found for tag {}, fetching from database", tag_id);
                let df = self.db.get_dataframe_for_tag(tag_id, range).await?;
                Ok(df)
            }
        }
    }

    pub async fn add_state(&self, value: &PersistentHomeStateValue, timestamp: &DateTime) -> Result<()> {
        self.db.add_state(value, timestamp).await
    }
}

impl<T> DataPointAccess<T> for T
where
    T: Into<PersistentHomeState> + Into<HomeState> + ValueObject + Clone,
{
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<T::ValueType>> {
        let channel: PersistentHomeState = self.clone().into();
        let tag_id = api.db.get_tag_id(channel.clone(), false).await?;

        api.get_datapoint(tag_id, &t!(now))
            .await
            .map(|dp| DataPoint::new(self.from_f64(dp.value), dp.timestamp))
    }
}

impl<T> DataFrameAccess<T> for T
where
    T: Into<PersistentHomeState> + ValueObject + Clone,
{
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<T::ValueType>> {
        let channel: PersistentHomeState = self.clone().into();
        let tag_id = api.db.get_tag_id(channel.clone(), false).await?;

        let df = api
            .get_dataframe(tag_id, &range)
            .await?
            .map(|dp| self.from_f64(dp.value));

        Ok(df)
    }
}

//
//COMMAND
//
impl HomeApi {
    async fn get_commands(&self, target: &CommandTarget, range: &DateTimeRange) -> Result<Vec<CommandExecution>> {
        match self.get_commands_from_cache(target, range).await {
            Some(commands) => Ok(commands
                .1
                .iter()
                .filter(|cmd| range.contains(&cmd.created))
                .cloned()
                .collect()),
            None => {
                tracing::warn!("No cached commands found for target {:?}, fetching from database", target);
                let commands = self.db.query_all_commands(Some(target.clone()), range).await?;
                Ok(commands)
            }
        }
    }

    pub async fn execute(
        &self,
        command: Command,
        source: crate::home::command::CommandSource,
    ) -> Result<CommandExecutionResult> {
        let target: CommandTarget = command.clone().into();
        let last_execution = self
            .get_latest_command(target, t!(48 hours ago))
            .await?
            .filter(|e| e.source == source && e.command == command)
            .map(|e| e.created);

        //wait until roundtrip is completed. State might not have been updated yet
        let was_just_executed = last_execution.is_some_and(|dt| dt > t!(30 seconds ago));

        if was_just_executed {
            return Ok(CommandExecutionResult::Skipped);
        }

        let was_latest_execution = last_execution.is_some();
        let is_reflected_in_state = command.is_reflected_in_state(self).await?;

        let result = if !was_latest_execution || !is_reflected_in_state {
            self.db
                .save_command(&command, source, TraceContext::current_correlation_id())
                .await?;
            Ok(CommandExecutionResult::Triggered)
        } else {
            Ok(CommandExecutionResult::Skipped)
        };

        // Invalidate command cache after saving command
        self.invalidate_command_cache(&command.into()).await;

        result
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
}

impl CommandExecutionAccess for HomeApi {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>> {
        let target = target.into();
        let range = DateTimeRange::new(since, t!(now));
        let commands = self.get_commands(&target, &range).await?;
        Ok(commands.into_iter().max_by_key(|cmd| cmd.created))
    }

    async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        let target = target.into();
        let range = DateTimeRange::new(since, t!(now));
        self.get_commands(&target, &range).await
    }
}

//
//USER TRIGGER
//
impl HomeApi {
    pub async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        self.db.add_user_trigger(trigger).await
    }

    pub async fn latest_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<DataPoint<UserTrigger>>> {
        self.db.latest_since(target, since).await
    }
}

//
//PLANNING TRACE
//
impl HomeApi {
    pub async fn add_planning_trace(&self, result: &PlanningTrace) -> anyhow::Result<()> {
        self.db.add_planning_trace(result).await
    }
}

//
// AVAILABILITY
//
impl HomeApi {
    pub async fn add_item_availability(&self, item: ItemAvailability) -> anyhow::Result<()> {
        self.db.add_item_availability(item).await
    }
}

//
// GRAFANA ONLY
//
impl HomeApi {
    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<PersistentHomeStateValue>>> {
        self.db.get_all_data_points_in_range(range).await
    }

    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> Result<Vec<CommandExecution>> {
        self.db.query_all_commands(None, &DateTimeRange::new(from, until)).await
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

#[cfg(test)]
mod tests {
    use super::*;

    impl HomeApi {
        pub fn for_testing() -> Self {
            let pool = sqlx::PgPool::connect_lazy("postgres://dummy:dummy@localhost/dummy").unwrap();
            Self::new(Database::new(pool))
        }

        pub fn with_fixed_current_dp<T>(&mut self, state: T, value: impl Into<T::ValueType>, timestamp: DateTime)
        where
            T: Into<HomeState> + ValueObject + Clone,
        {
            let value = value.into();
            self.state_dp_mock
                .insert(state.clone().into(), DataPoint::new(state.to_f64(&value), timestamp));
        }

        pub fn with_fixed_ts<T, V>(&mut self, state: T, values: &[(V, DateTime)])
        where
            T: Into<HomeState> + ValueObject + Clone,
            V: Into<T::ValueType> + Clone,
        {
            let dps: Vec<DataPoint<f64>> = values
                .iter()
                .map(|(v, ts)| DataPoint::new(state.to_f64(&v.clone().into()), *ts))
                .collect();
            let df = DataFrame::new(dps).expect("Error creating test timeseries");

            self.state_ts_mock.insert(state.into(), df);
        }

        pub fn get_fixed_current_dp<T>(&self, state: T) -> Option<DataPoint<T::ValueType>>
        where
            T: Into<HomeState> + ValueObject + Clone,
        {
            self.state_dp_mock
                .get(&state.clone().into())
                .map(|dp| DataPoint::new(state.from_f64(dp.value), dp.timestamp))
        }

        pub fn get_fixed_ts<T>(&self, state: T) -> Option<DataFrame<T::ValueType>>
        where
            T: Into<HomeState> + ValueObject + Clone,
        {
            self.state_ts_mock
                .get(&state.clone().into())
                .map(|df| df.map(|dp| state.from_f64(dp.value)))
        }
    }
}
