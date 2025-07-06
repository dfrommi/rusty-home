use crate::{
    core::timeseries::DataPoint,
    home::trigger::{UserTrigger, UserTriggerTarget},
};
use anyhow::Context;
use crate::t;
use crate::core::time::DateTime;

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
    pub async fn latest_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<DataPoint<UserTrigger>>> {
        let db_target = serde_json::json!(target);

        let rec = sqlx::query!(
            r#"SELECT trigger, timestamp FROM user_trigger
                WHERE trigger @> $1
                AND timestamp >= $2
                AND timestamp <= $3
                ORDER BY timestamp DESC
                LIMIT 1"#,
            db_target,
            since.into_db(),
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_optional(&self.pool)
        .await?;

        let result = match rec {
            Some(row) => Some(DataPoint::new(
                serde_json::from_value(row.trigger)?,
                row.timestamp.into(),
            )),
            None => None,
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::home::trigger::*;
    use crate::t;

    use crate::Database;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_read_write(pool: sqlx::PgPool) {
        let db = Database::new(pool);
        db.add_user_trigger(UserTrigger::Homekit(Homekit::InfraredHeaterPower(true)))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        db.add_user_trigger(UserTrigger::Homekit(Homekit::InfraredHeaterPower(false)))
            .await
            .unwrap();

        let latest_trigger = db
            .latest_since(
                &UserTriggerTarget::Homekit(HomekitTarget::InfraredHeaterPower),
                t!(10 seconds ago),
            )
            .await
            .unwrap();

        assert!(matches!(
            latest_trigger,
            Some(DataPoint {
                value: UserTrigger::Homekit(Homekit::InfraredHeaterPower(false)),
                ..
            })
        ));
    }
}
