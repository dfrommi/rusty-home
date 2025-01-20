use anyhow::Context;
use api::trigger::UserTrigger;
use infrastructure::monitoring;
use support::t;

use crate::{core::domain::UserTriggerStorage, Database};

impl UserTriggerStorage for Database {
    async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let trigger: serde_json::Value = serde_json::to_value(trigger)?;

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp, correlation_id) VALUES ($1, $2, $3)"#,
            trigger,
            t!(now).into_db(),
            monitoring::TraceContext::current_correlation_id(),
        )
        .execute(&self.db_pool)
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }
}
