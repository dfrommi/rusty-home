mod cache;

use super::persistence::Database;
use crate::t;

#[derive(Clone)]
pub struct HomeApi {
    _db: Database,
    cache: cache::HomeApiCache,
}

impl HomeApi {
    pub fn new(db: Database) -> Self {
        Self {
            cache: cache::HomeApiCache::new(cache::CachingRange::OfLast(t!(72 hours)), db.clone()),
            _db: db,
        }
    }
}

impl HomeApi {
    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        self.cache.invalidate_ts_cache(tag_id).await;
    }
}
