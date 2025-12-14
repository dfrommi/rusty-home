mod cache;

use super::persistence::Database;
use super::time::{DateTime, DateTimeRange};
use super::timeseries::{DataFrame, DataPoint};
use crate::core::id::ExternalId;
use crate::home::command::{Command, CommandExecution, CommandTarget};
use crate::t;
use crate::trigger::UserTriggerId;
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
    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        self.cache.invalidate_ts_cache(tag_id).await;
    }

    pub async fn invalidate_command_cache(&self, target: &CommandTarget) {
        self.cache.invalidate_command_cache(target).await;
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
}

//
// GRAFANA ONLY
//
impl HomeApi {
    pub async fn get_all_commands(&self, from: DateTime, until: DateTime) -> Result<Vec<CommandExecution>> {
        let commands = self
            .db
            .query_all_commands(None, &DateTimeRange::new(from, until))
            .await?;
        Ok(self.apply_timeshift_filter(commands, |cmd| cmd.created))
    }
}
