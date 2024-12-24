use anyhow::Context;
use api::trigger::{UserTrigger, UserTriggerTarget};
use sqlx::PgPool;
use support::{t, time::DateTime};

use crate::port::{UserTriggerAccess, UserTriggerExecutor};

impl<DB: AsRef<PgPool>> UserTriggerExecutor for DB {
    async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let trigger: serde_json::Value = serde_json::to_value(trigger)?;

        sqlx::query!(
            r#"INSERT INTO user_trigger (trigger, timestamp) VALUES ($1, $2)"#,
            trigger,
            t!(now).into_db(),
        )
        .execute(self.as_ref())
        .await
        .map(|_| ())
        .context("Error adding user trigger")
    }
}

impl<DB> UserTriggerAccess for DB
where
    DB: AsRef<PgPool>,
{
    async fn latest_since(
        &self,
        target: &UserTriggerTarget,
        since: DateTime,
    ) -> anyhow::Result<Option<UserTrigger>> {
        let db_target = serde_json::json!(target);

        let rec = sqlx::query!(
            r#"SELECT trigger FROM user_trigger
                WHERE trigger @> $1
                AND timestamp >= $2
                AND timestamp <= $3
                ORDER BY timestamp DESC
                LIMIT 1"#,
            db_target,
            since.into_db(),
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_optional(self.as_ref())
        .await?;

        match rec {
            Some(row) => Ok(Some(serde_json::from_value(row.trigger)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use api::trigger::*;
    use support::t;

    use crate::adapter::persistence::TestDb;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_read_write(pool: sqlx::PgPool) {
        let db = TestDb { pool };
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
            Some(UserTrigger::Homekit(Homekit::InfraredHeaterPower(false)))
        ));
    }
}
