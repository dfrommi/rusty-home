use anyhow::Context;
use api::trigger::UserTrigger;
use infrastructure::TraceContext;
use support::t;

use crate::Database;

impl Database {
    pub async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let trigger: serde_json::Value = serde_json::to_value(trigger)?;

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp, correlation_id) VALUES ($1, $2, $3)"#,
            trigger,
            t!(now).into_db(),
            TraceContext::current_correlation_id(),
        )
        .execute(&self.db_pool)
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }
}
