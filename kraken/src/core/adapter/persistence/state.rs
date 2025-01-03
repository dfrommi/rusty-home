use api::{
    get_tag_id,
    state::{db::DbValue, ChannelValue},
};

use anyhow::Result;
use support::time::DateTime;

use crate::{core::domain::StateStorage, Database};

impl StateStorage for Database {
    async fn add_state(&self, value: &ChannelValue, timestamp: &DateTime) -> Result<()> {
        let tags_id = get_tag_id(&self.db_pool, value.into(), true).await?;

        let fvalue: DbValue = value.into();

        sqlx::query!(
            r#"WITH latest_value AS (
                SELECT value
                FROM thing_value
                WHERE tag_id = $1
                ORDER BY timestamp DESC, id DESC
                LIMIT 1
            )
            INSERT INTO thing_value (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)"#,
            tags_id as i32,
            fvalue.as_ref(),
            timestamp.into_db()
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
