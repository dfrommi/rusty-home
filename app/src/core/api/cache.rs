use std::sync::Arc;

use moka::future::Cache;

use crate::{
    core::{
        persistence::Database,
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataFrame,
    },
    home::command::{CommandExecution, CommandTarget},
};

#[derive(Clone)]
pub struct HomeApiCache {
    db: Database,
    caching_range: CachingRange,
    ts_cache: Cache<i64, Arc<(DateTimeRange, DataFrame<f64>)>>,
    cmd_cache: Cache<CommandTarget, Arc<(DateTimeRange, Vec<CommandExecution>)>>,
}

#[derive(Debug, Clone)]
pub enum CachingRange {
    OfLast(Duration),
    Fixed(DateTimeRange),
}

impl HomeApiCache {
    pub fn new(caching_range: CachingRange, db: Database) -> Self {
        Self {
            db,
            caching_range,
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(3 * 60 * 60))
                .build(),
            cmd_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(3 * 60 * 60))
                .build(),
        }
    }

    pub fn is_covering(&self, range: &DateTimeRange) -> bool {
        self.caching_range().covers(range)
    }

    // Cache Management Methods
    fn caching_range(&self) -> DateTimeRange {
        //Caching always uses real time, never timeshifted. This allows stable caching while
        //shifting around
        match &self.caching_range {
            CachingRange::OfLast(duration) => {
                DateTimeRange::new(DateTime::real_now() - duration.clone(), DateTime::max_value())
            }
            CachingRange::Fixed(range) => range.clone(),
        }
    }

    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        tracing::debug!("Start preloading cache");

        let tag_ids = self.db.get_all_tag_ids().await?;
        let cache_range = self.caching_range();

        // Process all tag IDs in parallel
        let futures: Vec<_> = tag_ids
            .into_iter()
            .map(|tag_id| {
                let cache_range = cache_range.clone();
                async move {
                    self.get_dataframe_from_cache(tag_id, &cache_range).await;
                    tag_id
                }
            })
            .collect();

        // Wait for all parallel operations to complete
        let results = futures::future::join_all(futures).await;

        tracing::debug!("Preloading cache done for {} tags", results.len());
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
    pub async fn get_dataframe_from_cache(
        &self,
        tag_id: i64,
        range: &DateTimeRange,
    ) -> Option<Arc<(DateTimeRange, DataFrame<f64>)>> {
        let df = self
            .ts_cache
            .try_get_with(tag_id, async {
                tracing::debug!("No cached data found for tag {}, fetching from database", tag_id);
                let cache_range = self.caching_range();
                self.db.get_dataframe_for_tag(tag_id, &cache_range).await.map(|df| {
                    // Adjust cached range to actual data range, expanded by the cache range
                    let actual_data_range = df.range();
                    let effective_range = DateTimeRange::new(
                        *cache_range.start().min(actual_data_range.start()),
                        *cache_range.end().max(actual_data_range.end()),
                    );
                    Arc::new((effective_range, df))
                })
            })
            .await;

        match df {
            Ok(cached) if cached.0.covers(range) => Some(cached),
            Err(e) => {
                tracing::error!("Error fetching dataframe for tag {} from cache or init cacke: {:?}", tag_id, e);
                None
            }
            _ => None,
        }
    }

    pub async fn get_commands_from_cache(
        &self,
        target: &CommandTarget,
        range: &DateTimeRange,
    ) -> Option<Arc<(DateTimeRange, Vec<CommandExecution>)>> {
        let commands = self
            .cmd_cache
            .try_get_with(target.clone(), async {
                tracing::debug!("No command-cache entry found for target {:?}", target);
                let cache_range = self.caching_range();
                self.db
                    .query_all_commands(Some(target.clone()), &cache_range)
                    .await
                    .map(|cmds| {
                        // Adjust cached range to actual data range, expanded by the cache range
                        let effective_range = if cmds.is_empty() {
                            cache_range
                        } else {
                            let min_timestamp = cmds.iter().map(|cmd| cmd.created).min().unwrap();
                            let max_timestamp = cmds.iter().map(|cmd| cmd.created).max().unwrap();
                            let actual_data_range = DateTimeRange::new(min_timestamp, max_timestamp);
                            DateTimeRange::new(
                                *cache_range.start().min(actual_data_range.start()),
                                *cache_range.end().max(actual_data_range.end()),
                            )
                        };
                        Arc::new((effective_range, cmds))
                    })
            })
            .await;

        match commands {
            Ok(cached) if cached.0.covers(range) => Some(cached),
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
