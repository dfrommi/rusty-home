use std::sync::Arc;

use api::{
    command::{CommandExecution, CommandTarget},
    state::db::DbValue,
};
use moka::future::Cache;
use support::{
    t,
    time::{DateTime, DateTimeRange, Duration},
    DataFrame,
};

mod command;
mod planning_trace;
mod state;
mod trigger;

#[derive(Clone)]
pub struct Database {
    pool: sqlx::PgPool,
    cache_duration: Duration,
    ts_cache: Cache<i64, Arc<DataFrame<DbValue>>>,
    cmd_cache: Cache<CommandTarget, Arc<(DateTime, Vec<CommandExecution>)>>,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            cache_duration: t!(48 hours),
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(48 * 60 * 60))
                .build(),
            cmd_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(48 * 60 * 60))
                .build(),
        }
    }
}
