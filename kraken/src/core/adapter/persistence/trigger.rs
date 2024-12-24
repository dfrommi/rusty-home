use anyhow::Context;
use api::trigger::UserTrigger;
use support::t;

use crate::{core::domain::UserTriggerStorage, Database};

impl UserTriggerStorage for Database {
    async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let trigger: serde_json::Value = serde_json::to_value(trigger)?;

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp) VALUES ($1, $2)"#,
            trigger,
            t!(now).into_db(),
        )
        .execute(&self.db_pool)
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }
}
