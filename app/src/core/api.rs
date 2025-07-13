use super::ValueObject;
use super::persistence::{Database, OfflineItem};
use super::planner::PlanningTrace;
use super::time::{DateTime, DateTimeRange, Duration};
use super::timeseries::{DataFrame, DataPoint, TimeSeries, interpolate::Estimatable};
use crate::core::ItemAvailability;
use crate::home::command::{Command, CommandExecution, CommandTarget};
use crate::home::state::{HomeState, PersistentHomeState, PersistentHomeStateValue};
use crate::home::trigger::{UserTrigger, UserTriggerTarget};
use crate::port::{CommandExecutionAccess, CommandExecutionResult, DataPointAccess, TimeSeriesAccess};
use crate::t;
use anyhow::Result;
use infrastructure::TraceContext;
use r#macro::mockable;
use moka::future::Cache;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct HomeApi {
    db: Database,
    ts_cache_duration: Duration,
    ts_cache: Cache<i64, Arc<DataFrame<f64>>>,
    cmd_cache_duration: Duration,
    cmd_cache: Cache<CommandTarget, Arc<(DateTime, Vec<CommandExecution>)>>,
    #[cfg(test)]
    state_dp_mock: std::collections::HashMap<HomeState, DataPoint<f64>>,
    #[cfg(test)]
    state_ts_mock: std::collections::HashMap<HomeState, DataFrame<f64>>,
}

impl HomeApi {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            ts_cache_duration: t!(48 hours),
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(48 * 60 * 60))
                .build(),
            cmd_cache_duration: t!(72 hours),
            cmd_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(72 * 60 * 60))
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
    fn ts_caching_range(&self) -> DateTimeRange {
        let now = t!(now);
        DateTimeRange::new(now - self.ts_cache_duration.clone(), now)
    }

    fn cmd_caching_range(&self) -> DateTimeRange {
        let now = t!(now);
        DateTimeRange::new(now - self.cmd_cache_duration.clone(), now)
    }

    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        tracing::debug!("Start preloading cache");

        let tag_ids = self.db.get_all_tag_ids().await?;

        for tag_id in tag_ids {
            if let Err(e) = self.get_default_dataframe(tag_id).await {
                tracing::error!("Error preloading timeseries cache for tag {}: {:?}", tag_id, e);
            }
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
    async fn get_default_dataframe(&self, tag_id: i64) -> anyhow::Result<DataFrame<f64>> {
        let df = self
            .ts_cache
            .try_get_with(tag_id, async {
                tracing::debug!("No cached data found for tag {}, fetching from database", tag_id);
                let range = self.ts_caching_range();
                self.db.get_dataframe_for_tag(tag_id, &range).await.map(Arc::new)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Error initializing timeseries cache for tag {}: {:?}", tag_id, e))?;

        Ok((*df).clone())
    }
}

//
//STATE
//
impl HomeApi {
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

        let df: DataFrame<f64> = api.get_default_dataframe(tag_id).await?;

        match df.prev_or_at(t!(now)) {
            Some(dp) => Ok(dp.map_value(|&v| self.from_f64(v))),
            None => anyhow::bail!("No data found"),
        }
    }
}

impl<T> TimeSeriesAccess<T> for T
where
    //TODO into PersistentHomeState should automatically imply Into<HomeState>
    T: Into<PersistentHomeState> + Into<HomeState> + Estimatable + Clone + Debug,
{
    #[mockable]
    async fn series(&self, range: DateTimeRange, api: &HomeApi) -> Result<TimeSeries<T>> {
        let channel: PersistentHomeState = self.clone().into();
        let tag_id = api.db.get_tag_id(channel.clone(), false).await?;

        let df = api.get_default_dataframe(tag_id).await?.map(|dp| self.from_f64(dp.value));

        if range.start() < df.range().start() {
            tracing::warn!(
                "Timeseries out of cache range requested for item {:?} and range {}. Doing full query",
                tag_id,
                &range
            );

            let df = api
                .db
                .get_dataframe_for_tag(tag_id, &range)
                .await?
                .map(|dp| self.from_f64(dp.value));
            return TimeSeries::new(self.clone(), &df, range);
        }

        TimeSeries::new(self.clone(), &df, range)
    }
}

//
//COMMAND
//
impl HomeApi {
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
        let commands = self.get_commands_using_cache(&target, since).await?;
        Ok(commands.into_iter().max_by_key(|cmd| cmd.created))
    }

    async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        let target = target.into();
        self.get_commands_using_cache(&target, since).await
    }
}

impl HomeApi {
    async fn get_commands_using_cache(&self, target: &CommandTarget, since: DateTime) -> Result<Vec<CommandExecution>> {
        let cached = self
            .cmd_cache
            .try_get_with(target.clone(), async {
                tracing::debug!("No command-cache entry found for target {:?}", target);
                let range = self.cmd_caching_range();

                self.db
                    .query_all_commands(Some(target.clone()), range.start(), range.end())
                    .await
                    .map(|cmds| Arc::new((*range.start(), cmds)))
            })
            .await
            .map_err(|e| anyhow::anyhow!("Error initializing command cache for target {:?}: {:?}", target, e))?;

        if since < cached.0 {
            tracing::info!(
                ?since,
                offset = %since.elapsed().to_iso_string(),
                cache_start = %cached.0,
                "Requested time range is before cached commands, querying database"
            );
            return self.db.query_all_commands(Some(target.clone()), &since, &t!(now)).await;
        }

        let commands: Vec<CommandExecution> = cached.1.iter().filter(|&cmd| cmd.created >= since).cloned().collect();

        Ok(commands)
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
        self.db.query_all_commands(None, &from, &until).await
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
                .insert(state.into(), DataPoint::new(state.to_f64(&value), timestamp));
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
                .get(&state.into())
                .map(|dp| DataPoint::new(state.from_f64(dp.value), dp.timestamp))
        }

        pub fn get_fixed_ts<T>(&self, state: T) -> Option<DataFrame<T::ValueType>>
        where
            T: Into<HomeState> + ValueObject + Clone,
        {
            self.state_ts_mock
                .get(&state.into())
                .map(|df| df.map(|dp| state.from_f64(dp.value)))
        }
    }
}
