mod command;
mod energy_reading;
mod state;

#[allow(unused_imports)]
pub use energy_reading::*;

pub use command::NewCommandAvailablePgListener;
use sqlx::PgPool;

pub struct Database {
    db_pool: PgPool,
}

impl Database {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}
