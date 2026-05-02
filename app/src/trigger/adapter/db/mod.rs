use std::collections::HashSet;

use crate::core::time::{DateTime, DateTimeRange};
use crate::t;
use crate::trigger::{UserTrigger, UserTriggerExecution, UserTriggerId};
use anyhow::Context;
use serde_json::Value;

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
            infrastructure::TraceContext::current()
                .correlation_id()
                .map(|id| id.to_string()),
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_triggers_active_from_if_unset(&self, trigger_ids: &[UserTriggerId]) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"UPDATE user_trigger
               SET active_from = $2
               WHERE id = ANY($1)
               AND active_from IS NULL"#,
            trigger_ids as &[UserTriggerId],
            t!(now).into_db(),
        )
        .execute(&self.pool)
        .await
        .context("Error setting user trigger active_from")?;

        Ok(result.rows_affected())
    }

    pub async fn get_all_triggers_active_anytime_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let records = sqlx::query!(
            r#"SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", active_from as "active_from", active_until as "active_until", correlation_id
                    FROM user_trigger
                    WHERE (timestamp >= $1 AND timestamp <= $2)
                    OR (active_until IS NOT NULL AND active_until >= $1 AND active_until <= $2) 
                    OR (timestamp <= $2 AND (active_until IS NULL OR active_until >= $2))
               ORDER BY timestamp DESC"#,
            range.start().into_db(),
            range.end().into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(records.len());

        for row in records {
            let Some(trigger) = parse_user_trigger(row.id, row.trigger) else {
                continue;
            };
            let timestamp = row.timestamp.into();
            let active_from = row.active_from.map(|dt| dt.into());
            let active_until = row.active_until.map(|dt| dt.into());

            result.push(UserTriggerExecution {
                id: row.id.into(),
                trigger,
                timestamp,
                active_from,
                active_until,
                correlation_id: row.correlation_id,
            });
        }

        Ok(result)
    }

    pub async fn get_all_active_triggers_since(&self, since: DateTime) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let now = t!(now);

        let records = sqlx::query!(
            r#"SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", active_from as "active_from", active_until as "active_until", correlation_id
                    FROM user_trigger
                    WHERE timestamp >= $1
                    AND timestamp <= $2
                    AND (active_until IS NULL OR active_until >= $2)
               ORDER BY timestamp DESC"#,
            since.into_db(),
            now.into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        //Only take latest per target
        let mut seen_targets = HashSet::new();
        let mut result = Vec::with_capacity(records.len());

        for row in records {
            let Some(trigger) = parse_user_trigger(row.id, row.trigger) else {
                continue;
            };
            let timestamp = row.timestamp.into();
            let active_from = row.active_from.map(|dt| dt.into());
            let active_until = row.active_until.map(|dt| dt.into());

            let target = trigger.target();
            if seen_targets.contains(&target) {
                continue;
            }
            seen_targets.insert(target);

            result.push(UserTriggerExecution {
                id: row.id.into(),
                trigger,
                timestamp,
                active_from,
                active_until,
                correlation_id: row.correlation_id,
            });
        }

        Ok(result)
    }
}

fn parse_user_trigger(id: i64, trigger: Value) -> Option<UserTrigger> {
    match serde_json::from_value(trigger) {
        Ok(trigger) => Some(trigger),
        Err(e) => {
            tracing::warn!("Invalid user_trigger row with id {}, ignoring: {}", id, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{core::time::DateTimeRange, t, trigger::OnOffDevice};

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn active_trigger_query_ignores_unsupported_trigger_json(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = TriggerRepository::new(pool);

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp) VALUES ($1, $2), ($3, $4)"#,
            serde_json::json!({
                "type": "removed_trigger",
                "device": "removed_device",
                "on": true
            }),
            t!(30 minutes ago).into_db(),
            serde_json::json!(UserTrigger::DevicePower {
                device: OnOffDevice::InfraredHeater,
                on: true,
            }),
            t!(20 minutes ago).into_db(),
        )
        .execute(&repo.pool)
        .await?;

        let triggers = repo.get_all_active_triggers_since(t!(1 hours ago)).await?;

        assert_eq!(triggers.len(), 1);
        match &triggers[0].trigger {
            UserTrigger::DevicePower { device, on } => {
                assert_eq!(device, &OnOffDevice::InfraredHeater);
                assert!(*on);
            }
            other => panic!("Unexpected trigger: {other:?}"),
        }

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn range_trigger_query_ignores_unsupported_trigger_json(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = TriggerRepository::new(pool);

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp) VALUES ($1, $2), ($3, $4)"#,
            serde_json::json!({
                "type": "removed_trigger",
                "device": "removed_device",
                "on": true
            }),
            t!(30 minutes ago).into_db(),
            serde_json::json!(UserTrigger::DevicePower {
                device: OnOffDevice::InfraredHeater,
                on: true,
            }),
            t!(20 minutes ago).into_db(),
        )
        .execute(&repo.pool)
        .await?;

        let triggers = repo
            .get_all_triggers_active_anytime_in_range(DateTimeRange::since(t!(1 hours ago)))
            .await?;

        assert_eq!(triggers.len(), 1);
        match &triggers[0].trigger {
            UserTrigger::DevicePower { device, on } => {
                assert_eq!(device, &OnOffDevice::InfraredHeater);
                assert!(*on);
            }
            other => panic!("Unexpected trigger: {other:?}"),
        }

        Ok(())
    }
}
