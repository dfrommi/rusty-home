use crate::core::time::{DateTime, DateTimeRange};
use crate::t;
use crate::trigger::{UserTrigger, UserTriggerExecution, UserTriggerId, UserTriggerTarget};
use anyhow::Context;

pub struct TriggerRepository {
    pool: sqlx::PgPool,
}

impl TriggerRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(skip(self))]
    pub async fn cancel_triggers_before_excluding(
        &self,
        before: DateTime,
        exclude_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"UPDATE user_trigger
               SET active_until = $1
               WHERE active_until IS NULL
               AND timestamp < $1
               AND id != ALL($2)"#,
            before.into_db(),
            exclude_ids as &[UserTriggerId],
        )
        .execute(&self.pool)
        .await
        .context("Error cancelling user triggers")?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let trigger: serde_json::Value = serde_json::to_value(trigger)?;

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp, correlation_id) VALUES ($1, $2, $3)"#,
            trigger,
            t!(now).into_db(),
            infrastructure::TraceContext::current_correlation_id(),
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }

    pub async fn user_triggers_in_range(
        &self,
        target: &UserTriggerTarget,
        range: &DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let db_target = serde_json::json!(target);
        let now = t!(now);

        let records = sqlx::query!(
            r#"(SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp >= $2
                    AND timestamp <= $3
                    AND (active_until IS NULL OR active_until >= $4))
               UNION ALL
               (SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp < $2
                    AND (active_until IS NULL OR active_until >= $4)
                    ORDER BY timestamp DESC
                    LIMIT 1)
               UNION ALL
               (SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp > $3
                    AND (active_until IS NULL OR active_until >= $4)
                    ORDER BY timestamp ASC
                    LIMIT 1)
               ORDER BY 2 ASC"#,
            db_target,
            range.start().into_db(),
            range.end().into_db(),
            now.into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(records.len());

        for row in records {
            let trigger: UserTrigger = serde_json::from_value(row.trigger)?;
            let timestamp = row.timestamp.into();
            result.push(UserTriggerExecution {
                id: row.id.into(),
                trigger,
                timestamp,
                correlation_id: row.correlation_id,
            });
        }

        Ok(result)
    }

    pub async fn get_all_active_triggers_since(&self, since: DateTime) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let now = t!(now);

        let records = sqlx::query!(
            r#"SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE timestamp >= $1
                    AND timestamp <= $2
                    AND (active_until IS NULL OR active_until >= $2)
               ORDER BY 2 ASC"#,
            since.into_db(),
            now.into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(records.len());

        for row in records {
            let trigger: UserTrigger = serde_json::from_value(row.trigger)?;
            let timestamp = row.timestamp.into();
            result.push(UserTriggerExecution {
                id: row.id.into(),
                trigger,
                timestamp,
                correlation_id: row.correlation_id,
            });
        }

        Ok(result)
    }

    pub async fn user_trigger_target_by_id(&self, id: &UserTriggerId) -> anyhow::Result<Option<UserTriggerTarget>> {
        let now = t!(now);
        let record = sqlx::query!(
            r#"SELECT trigger FROM user_trigger WHERE id = $1 AND (active_until IS NULL OR active_until >= $2)"#,
            id as &UserTriggerId,
            now.into_db()
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(row) => {
                let trigger: UserTrigger = serde_json::from_value(row.trigger)?;
                Ok(Some(trigger.target()))
            }
            None => Ok(None),
        }
    }
}
