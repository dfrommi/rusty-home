use std::sync::Arc;

use api::state::db::DbValue;
use moka::future::Cache;
use support::{t, time::Duration, DataFrame};

mod command;
mod planning_trace;
mod state;
mod trigger;

#[derive(Clone, derive_more::AsRef)]
pub struct Database {
    pool: sqlx::PgPool,
    ts_cache_duration: Duration,
    ts_cache: Cache<i64, Arc<DataFrame<DbValue>>>,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            ts_cache_duration: t!(48 hours),
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(48 * 60 * 60))
                .build(),
        }
    }
}
