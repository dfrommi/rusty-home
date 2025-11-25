use crate::core::time::{DateTime, DateTimeRange};
use crate::t;
use crate::{
    core::timeseries::DataPoint,
    home::trigger::{UserTrigger, UserTriggerId, UserTriggerTarget},
};
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct UserTriggerRequest {
    pub id: UserTriggerId,
    pub trigger: UserTrigger,
    pub timestamp: DateTime,
    pub correlation_id: Option<String>,
}

impl UserTriggerRequest {
    pub fn target(&self) -> UserTriggerTarget {
        self.trigger.target()
    }

    pub fn into_datapoint(self) -> DataPoint<UserTrigger> {
        DataPoint::new(self.trigger, self.timestamp)
    }

    pub fn to_datapoint(&self) -> DataPoint<UserTrigger> {
        DataPoint::new(self.trigger.clone(), self.timestamp)
    }
}

// User Trigger Management
// Methods for storing and retrieving user-triggered events and interactions
impl super::Database {
    #[tracing::instrument(skip(self))]
    pub async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
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

    #[tracing::instrument(name = "get_latest_user_trigger", skip(self))]
    pub async fn latest_trigger_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<DataPoint<UserTrigger>>> {
        let db_target = serde_json::json!(target);

        let rec = sqlx::query!(
            r#"SELECT trigger, timestamp FROM user_trigger
                WHERE trigger @> $1
                AND timestamp >= $2
                ORDER BY timestamp DESC
                LIMIT 1"#,
            db_target,
            since.into_db(),
        )
        .fetch_optional(&self.pool)
        .await?;

        let result = match rec {
            Some(row) => Some(DataPoint::new(serde_json::from_value(row.trigger)?, row.timestamp.into())),
            None => None,
        };

        Ok(result)
    }

    pub async fn user_triggers_in_range(
        &self,
        target: &UserTriggerTarget,
        range: &DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerRequest>> {
        let db_target = serde_json::json!(target);

        let records = sqlx::query!(
            r#"(SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp >= $2
                    AND timestamp <= $3)
               UNION ALL
               (SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp < $2
                    ORDER BY timestamp DESC
                    LIMIT 1)
               UNION ALL
               (SELECT id as "id!", trigger as "trigger!", timestamp as "timestamp!", correlation_id
                    FROM user_trigger
                    WHERE trigger @> $1
                    AND timestamp > $3
                    ORDER BY timestamp ASC
                    LIMIT 1)
               ORDER BY 2 ASC"#,
            db_target,
            range.start().into_db(),
            range.end().into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(records.len());

        for row in records {
            let trigger: UserTrigger = serde_json::from_value(row.trigger)?;
            let timestamp = row.timestamp.into();
            result.push(UserTriggerRequest {
                id: row.id.into(),
                trigger,
                timestamp,
                correlation_id: row.correlation_id,
            });
        }

        Ok(result)
    }

    pub async fn user_trigger_target_by_id(&self, id: &UserTriggerId) -> anyhow::Result<Option<UserTriggerTarget>> {
        let record = sqlx::query!(r#"SELECT trigger FROM user_trigger WHERE id = $1"#, id as &UserTriggerId)
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

#[cfg(test)]
mod tests {
    use crate::adapter::homekit::HomekitCommand;
    use crate::adapter::homekit::HomekitCommandTarget;
    use crate::home::trigger::*;
    use crate::t;

    use crate::Database;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_read_write(pool: sqlx::PgPool) {
        let db = Database::new(pool);
        db.add_user_trigger(UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(true)))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        db.add_user_trigger(UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(false)))
            .await
            .unwrap();

        let latest_trigger = db
            .latest_trigger_since(
                &UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower),
                t!(10 seconds ago),
            )
            .await
            .unwrap();

        assert!(matches!(
            latest_trigger,
            Some(DataPoint {
                value: UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(false)),
                ..
            })
        ));
    }
}
