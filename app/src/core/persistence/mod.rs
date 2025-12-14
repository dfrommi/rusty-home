mod command;
pub mod listener;

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}
