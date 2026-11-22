mod sqlite;

use std::path::PathBuf;

use anyhow::Context as _;
use serde::Deserialize;

pub use sqlite::SQLiteDatabase;

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("rusty_home.sqlite")
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    url: String,
    #[serde(default = "default_sqlite_path")]
    sqlite_path: PathBuf,
}

impl DatabaseConfig {
    pub fn new_sqlite_database(&self) -> anyhow::Result<SQLiteDatabase> {
        SQLiteDatabase::open(&self.sqlite_path)
    }

    pub async fn new_pool(&self) -> anyhow::Result<sqlx::PgPool> {
        sqlx::postgres::PgPoolOptions::new()
            .min_connections(2)
            .max_connections(8)
            .connect(&self.url)
            .await
            .with_context(|| format!("Error connecting to database {}", self.url))
    }

    pub async fn new_listener(&self) -> anyhow::Result<sqlx::postgres::PgListener> {
        sqlx::postgres::PgListener::connect(&self.url)
            .await
            .with_context(|| format!("Error connecting to database {}", self.url))
    }
}
