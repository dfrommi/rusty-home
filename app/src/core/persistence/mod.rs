mod availability;
mod command;
pub mod listener;
mod planning_trace;
mod state;
mod trigger;

pub use availability::OfflineItem;

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}
