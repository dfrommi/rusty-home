use crate::core::{
    persistence::Database,
    time::{DateTimeRange, Duration},
};

#[derive(Clone)]
pub struct HomeApiCache {
    _db: Database,
    _caching_range: CachingRange,
}

#[derive(Debug, Clone)]
pub enum CachingRange {
    OfLast(Duration),
    Fixed(DateTimeRange),
}

impl HomeApiCache {
    pub fn new(caching_range: CachingRange, db: Database) -> Self {
        Self {
            _db: db,
            _caching_range: caching_range,
        }
    }

    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        tracing::debug!("Invalidating timeseries cache for tag {}", tag_id);
        let _ = tag_id;
    }
}
