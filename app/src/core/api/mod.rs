mod cache;

use super::persistence::{Database, OfflineItem, UserTriggerRequest};
use super::time::{DateTime, DateTimeRange};
use super::timeseries::{DataFrame, DataPoint};
use crate::core::id::ExternalId;
use crate::home::availability::ItemAvailability;
use crate::home::command::{Command, CommandExecution, CommandTarget};
use crate::home::state::{PersistentHomeState, PersistentHomeStateTypeInfo, PersistentHomeStateValue};
use crate::home::trigger::{UserTrigger, UserTriggerId, UserTriggerTarget};
use crate::t;
use anyhow::Result;
use infrastructure::TraceContext;

#[derive(Clone)]
pub struct HomeApi {
    db: Database,
    cache: cache::HomeApiCache,
}

impl HomeApi {
    pub fn new(db: Database) -> Self {
        Self {
            cache: cache::HomeApiCache::new(cache::CachingRange::OfLast(t!(72 hours)), db.clone()),
            db,
        }
    }

    pub fn for_processing_of_range(&self, range: DateTimeRange) -> Self {
        if self.cache.is_covering(&range) {
            self.clone()
        } else {
            Self {
                cache: cache::HomeApiCache::new(cache::CachingRange::Fixed(range), self.db.clone()),
                db: self.db.clone(),
            }
        }
    }

    // Helper method to apply timeshift filtering to results
    fn apply_timeshift_filter<T>(&self, items: Vec<T>, get_timestamp: impl Fn(&T) -> DateTime) -> Vec<T> {
        if DateTime::is_shifted() {
            let now = t!(now);
            items.into_iter().filter(|item| get_timestamp(item) <= now).collect()
        } else {
            items
        }
    }

    // Helper method to apply timeshift filtering to DataFrames
    fn apply_timeshift_filter_to_dataframe(&self, df: DataFrame<f64>) -> Result<DataFrame<f64>> {
        if DateTime::is_shifted() {
            let now = t!(now);
            let filtered_points: Vec<DataPoint<f64>> = df.iter().filter(|dp| dp.timestamp <= now).cloned().collect();
            DataFrame::new(filtered_points)
        } else {
            Ok(df)
        }
    }
}

//
// CACHING
//
impl HomeApi {
    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        self.cache.preload_ts_cache().await?;
        self.cache.preload_user_trigger_cache().await
    }

    pub async fn preload_user_trigger_cache(&self) -> anyhow::Result<()> {
        self.cache.preload_user_trigger_cache().await
    }

    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        self.cache.invalidate_ts_cache(tag_id).await;
    }

    pub async fn invalidate_command_cache(&self, target: &CommandTarget) {
        self.cache.invalidate_command_cache(target).await;
    }

    pub async fn invalidate_user_trigger_cache(&self, target: &UserTriggerTarget) {
        self.cache.invalidate_user_trigger_cache(target).await;
    }

    pub async fn invalidate_user_trigger_cache_by_id(&self, id: &UserTriggerId) -> anyhow::Result<()> {
        match self.db.user_trigger_target_by_id(id).await? {
            Some(target) => self.invalidate_user_trigger_cache(&target).await,
            None => tracing::warn!("Can not invalidate user trigger cache, id {} not found", id),
        }
        Ok(())
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
        let df = match self.cache.get_dataframe_from_cache(tag_id, range).await {
            Some(df) => df.data().retain_range_with_context(range)?,
            None => {
                tracing::warn!("No cached data found for tag {}, fetching from database", tag_id);
                self.db.get_dataframe_for_tag(tag_id, range).await?
            }
        };
        self.apply_timeshift_filter_to_dataframe(df)
    }

    pub async fn current_data_point<T>(&self, item: &T) -> Result<DataPoint<T::ValueType>>
    where
        T: Into<PersistentHomeState> + PersistentHomeStateTypeInfo + Clone,
    {
        let channel: PersistentHomeState = item.clone().into();
        let tag_id = self.db.get_tag_id(channel.clone(), false).await?;

        self.get_datapoint(tag_id, &t!(now))
            .await
            .map(|dp| DataPoint::new(<T as PersistentHomeStateTypeInfo>::from_f64(item, dp.value), dp.timestamp))
    }

    pub async fn get_data_frame<T>(&self, item: &T, range: DateTimeRange) -> Result<DataFrame<T::ValueType>>
    where
        T: Into<PersistentHomeState> + PersistentHomeStateTypeInfo + Clone,
    {
        let channel: PersistentHomeState = item.clone().into();
        let tag_id = self.db.get_tag_id(channel.clone(), false).await?;

        let df = self
            .get_dataframe(tag_id, &range)
            .await?
            .map(|dp| <T as PersistentHomeStateTypeInfo>::from_f64(item, dp.value));

        Ok(df)
    }

    pub async fn add_state(&self, value: &PersistentHomeStateValue, timestamp: &DateTime) -> Result<()> {
        self.db.add_state(value, timestamp).await
    }
}

//
//COMMAND
//
impl HomeApi {
    async fn get_commands(&self, target: &CommandTarget, range: &DateTimeRange) -> Result<Vec<CommandExecution>> {
        let commands = match self.cache.get_commands_from_cache(target, range).await {
            Some(commands) => commands
                .data()
                .iter()
                .filter(|cmd| range.contains(&cmd.created))
                .cloned()
                .collect(),
            None => {
                tracing::warn!("No cached commands found for target {:?}, fetching from database", target);
                self.db.query_all_commands(Some(target.clone()), range).await?
            }
        };
        Ok(self.apply_timeshift_filter(commands, |cmd| cmd.created))
    }

    pub async fn save_command(
        &self,
        command: Command,
        action_id: &ExternalId,
        user_trigger_id: Option<UserTriggerId>,
    ) -> Result<()> {
        self.db
            .save_command(&command, action_id, user_trigger_id, TraceContext::current_correlation_id())
            .await?;

        // Invalidate command cache after saving command
        self.invalidate_command_cache(&command.into()).await;

        Ok(())
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

    pub async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>> {
        let target = target.into();
        let range = DateTimeRange::new(since, t!(now));
        let commands = self.get_commands(&target, &range).await?;
        Ok(commands.into_iter().max_by_key(|cmd| cmd.created))
    }

    pub async fn get_all_commands_for_target(
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
    async fn get_user_triggers(
        &self,
        target: &UserTriggerTarget,
        range: &DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerRequest>> {
        let triggers = match self.cache.get_user_triggers_from_cache(target, range).await {
            Some(cached) => cached
                .data()
                .iter()
                .filter(|req| range.contains(&req.timestamp))
                .cloned()
                .collect(),
            None => {
                tracing::warn!("No cached user triggers found for target {:?}, fetching from database", target);
                self.db.user_triggers_in_range(target, range).await?
            }
        };

        Ok(self.apply_timeshift_filter(triggers, |req| req.timestamp))
    }

    pub async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let target = trigger.target();
        self.db.add_user_trigger(trigger).await?;
        self.invalidate_user_trigger_cache(&target).await;
        Ok(())
    }

    pub async fn latest_trigger_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<UserTriggerRequest>> {
        let now = t!(now);
        let range = DateTimeRange::new(since, now);
        let triggers = self.get_user_triggers(target, &range).await?;

        Ok(triggers.into_iter().max_by_key(|it| it.timestamp))
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
        let data_points = self.db.get_all_data_points_in_range(range).await?;
        Ok(self.apply_timeshift_filter(data_points, |dp| dp.timestamp))
    }

    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> Result<Vec<CommandExecution>> {
        let commands = self
            .db
            .query_all_commands(None, &DateTimeRange::new(from, until))
            .await?;
        Ok(self.apply_timeshift_filter(commands, |cmd| cmd.created))
    }

    pub async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        self.db.get_offline_items().await
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     impl HomeApi {
//         pub fn for_testing() -> Self {
//             let pool = sqlx::PgPool::connect_lazy("postgres://dummy:dummy@localhost/dummy").unwrap();
//             Self::new(Database::new(pool))
//         }
//
//         pub fn with_fixed_current_dp<T>(&mut self, state: T, value: impl Into<T::ValueType>, timestamp: DateTime)
//         where
//             T: Into<HomeState> + ValueObject + Clone,
//         {
//             let value = value.into();
//             self.state_dp_mock
//                 .insert(state.clone().into(), DataPoint::new(state.to_f64(&value), timestamp));
//         }
//
//         pub fn with_fixed_df<T, V>(&mut self, state: T, values: &[(V, DateTime)])
//         where
//             T: Into<HomeState> + ValueObject + Clone,
//             V: Into<T::ValueType> + Clone,
//         {
//             let dps: Vec<DataPoint<f64>> = values
//                 .iter()
//                 .map(|(v, ts)| DataPoint::new(state.to_f64(&v.clone().into()), *ts))
//                 .collect();
//             let df = DataFrame::new(dps).expect("Error creating test timeseries");
//
//             self.state_df_mock.insert(state.into(), df);
//         }
//
//         pub fn get_fixed_current_dp<T>(&self, state: T) -> Option<DataPoint<T::ValueType>>
//         where
//             T: Into<HomeState> + ValueObject + Clone,
//         {
//             self.state_dp_mock
//                 .get(&state.clone().into())
//                 .map(|dp| DataPoint::new(state.from_f64(dp.value), dp.timestamp))
//         }
//
//         pub fn get_fixed_df<T>(&self, state: T) -> Option<DataFrame<T::ValueType>>
//         where
//             T: Into<HomeState> + ValueObject + Clone,
//         {
//             self.state_df_mock
//                 .get(&state.clone().into())
//                 .map(|df| df.map(|dp| state.from_f64(dp.value)))
//         }
//     }
// }
