mod availability;
mod command;
pub mod listener;
mod planning_trace;
mod state;
mod trigger;

pub use state::DbValue;

use std::sync::Arc;

use crate::home::command::{CommandExecution, CommandTarget};
use moka::future::Cache;
use support::{
    DataFrame, t,
    time::{DateTime, Duration},
};

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
    ts_cache_duration: Duration,
    ts_cache: Cache<i64, Arc<DataFrame<DbValue>>>,
    cmd_cache_duration: Duration,
    cmd_cache: Cache<CommandTarget, Arc<(DateTime, Vec<CommandExecution>)>>,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            ts_cache_duration: t!(48 hours),
            ts_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(48 * 60 * 60))
                .build(),
            cmd_cache_duration: t!(72 hours),
            cmd_cache: Cache::builder()
                .time_to_live(std::time::Duration::from_secs(72 * 60 * 60))
                .build(),
        }
    }
}
