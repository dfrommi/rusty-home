mod sqlite;

use std::path::PathBuf;

use serde::Deserialize;

pub use sqlite::SQLiteDatabase;

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("rusty_home.sqlite")
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_sqlite_path")]
    sqlite_path: PathBuf,
}

impl DatabaseConfig {
    pub fn new_sqlite_database(&self) -> anyhow::Result<SQLiteDatabase> {
        SQLiteDatabase::open(&self.sqlite_path)
    }
}
