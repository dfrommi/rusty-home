use anyhow::Context as _;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    url: String,
}

impl DatabaseConfig {
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
