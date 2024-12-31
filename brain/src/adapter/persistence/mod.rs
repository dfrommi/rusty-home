use api::state::db::DbValue;
use moka::future::Cache;
use support::{time::Duration, DataPoint};

mod command;
mod planning_trace;
mod state;
mod trigger;

#[cfg(test)]
#[derive(derive_more::AsRef)]
struct TestDb {
    pool: sqlx::PgPool,
}

#[derive(Clone, derive_more::AsRef)]
pub struct Database {
    pool: sqlx::PgPool,
    cache: Option<Cache<i64, DataPoint<DbValue>>>,
}

impl Database {
    pub fn new(pool: sqlx::PgPool, cache_duration: Option<Duration>) -> Self {
        let cache = cache_duration.map(|duration| {
            Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(duration.as_secs() as u64))
                .build()
        });

        Self { pool, cache }
    }
}
