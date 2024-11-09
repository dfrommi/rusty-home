use api::{
    get_tag_id,
    state::{db::DbValue, ChannelValue},
};
use chrono::{DateTime, Utc};

use anyhow::Result;

use super::BackendApi;

pub trait StateRepository {
    async fn add_thing_value(&self, value: &ChannelValue, timestamp: &DateTime<Utc>) -> Result<()>;
}

impl StateRepository for BackendApi {
    async fn add_thing_value(&self, value: &ChannelValue, timestamp: &DateTime<Utc>) -> Result<()> {
        let tags_id = get_tag_id(&self.db_pool, value.into(), true).await?;

        let fvalue: DbValue = value.into();

        sqlx::query!(
            r#"WITH latest_value AS (
                SELECT value
                FROM thing_values
                WHERE tag_id = $1
                ORDER BY timestamp DESC, id DESC
                LIMIT 1
            )
            INSERT INTO thing_values (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)"#,
            tags_id,
            fvalue.as_ref(),
            timestamp
        )
        .execute(&self.db_pool)
        .await?;

        //info!("Inserted new value: {:?}", event);

        Ok(())
    }
}
