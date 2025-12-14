use std::sync::Arc;

use moka::future::Cache;

use crate::{
    core::{
        persistence::{Database, UserTriggerRequest},
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataFrame,
    },
    home::{
        command::{CommandExecution, CommandTarget},
        trigger::UserTriggerTarget,
    },
};

#[derive(Clone)]
pub struct HomeApiCache {
    db: Database,
    caching_range: CachingRange,
    ts_cache: Cache<i64, CacheSlice<DataFrame<f64>>>,
    cmd_cache: Cache<CommandTarget, CacheSlice<Vec<CommandExecution>>>,
    user_trigger_cache: Cache<UserTriggerTarget, CacheSlice<Vec<UserTriggerRequest>>>,
}

#[derive(Debug, Clone)]
pub struct CacheSlice<T> {
    range: DateTimeRange,
    data: Arc<T>,
}

impl<T> CacheSlice<T> {
    fn new(range: DateTimeRange, data: T) -> Self {
        Self {
            range,
            data: Arc::new(data),
        }
    }

    fn covers(&self, range: &DateTimeRange) -> bool {
        self.range.covers(range)
    }

    pub fn data(&self) -> &T {
        self.data.as_ref()
    }
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
            user_trigger_cache: Cache::builder()
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

    pub async fn preload_user_trigger_cache(&self) -> anyhow::Result<()> {
        tracing::debug!("Start preloading user trigger cache");

        let targets = UserTriggerTarget::variants();
        let cache_range = self.caching_range();

        let futures: Vec<_> = targets
            .into_iter()
            .map(|target| {
                let cache_range = cache_range.clone();
                async move {
                    self.get_user_triggers_from_cache(&target, &cache_range).await;
                    target
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        tracing::debug!("Preloading user trigger cache done for {} targets", results.len());
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

    pub async fn invalidate_user_trigger_cache(&self, target: &UserTriggerTarget) {
        tracing::debug!("Invalidating user trigger cache for target {:?}", target);
        self.user_trigger_cache.invalidate(target).await;
    }

    pub async fn get_commands_from_cache(
        &self,
        target: &CommandTarget,
        range: &DateTimeRange,
    ) -> Option<CacheSlice<Vec<CommandExecution>>> {
        let commands = self
            .cmd_cache
            .try_get_with(target.clone(), async {
                let cache_range = self.caching_range();
                self.db
                    .query_all_commands(Some(target.clone()), &cache_range)
                    .await
                    .map(|cmds| {
                        // Adjust cached range to actual data range, expanded by the cache range
                        let effective_range = Self::merge_ranges(&cache_range, cmds.iter().map(|cmd| cmd.created));
                        CacheSlice::new(effective_range, cmds)
                    })
            })
            .await;

        match commands {
            Ok(cached) if cached.covers(range) => Some(cached),
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

    pub async fn get_user_triggers_from_cache(
        &self,
        target: &UserTriggerTarget,
        range: &DateTimeRange,
    ) -> Option<CacheSlice<Vec<UserTriggerRequest>>> {
        let triggers = self
            .user_trigger_cache
            .try_get_with(target.clone(), async {
                let cache_range = self.caching_range();
                self.db
                    .user_triggers_in_range(target, &cache_range)
                    .await
                    .map(|entries| {
                        let effective_range = Self::merge_ranges(&cache_range, entries.iter().map(|req| req.timestamp));
                        CacheSlice::new(effective_range, entries)
                    })
            })
            .await;

        match triggers {
            Ok(cached) if cached.covers(range) => Some(cached),
            Err(e) => {
                tracing::error!(
                    "Error fetching user triggers for target {:?} from cache or init cache: {:?}",
                    target,
                    e
                );
                None
            }
            _ => None,
        }
    }

    fn merge_ranges<I>(cache_range: &DateTimeRange, timestamps: I) -> DateTimeRange
    where
        I: Iterator<Item = DateTime>,
    {
        let mut min_ts = *cache_range.start();
        let mut max_ts = *cache_range.end();

        for ts in timestamps {
            if ts < min_ts {
                min_ts = ts;
            }
            if ts > max_ts {
                max_ts = ts;
            }
        }

        DateTimeRange::new(min_ts, max_ts)
    }

    fn extend_range(a: &DateTimeRange, b: &DateTimeRange) -> DateTimeRange {
        DateTimeRange::new(std::cmp::min(*a.start(), *b.start()), std::cmp::max(*a.end(), *b.end()))
    }
}
