use std::collections::HashSet;

use anyhow::Context as _;
use infrastructure::{EventEmitter, SQLiteDatabase, TraceContext};

use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
    trigger::{
        TriggerEvent, UserTrigger, UserTriggerExecution, UserTriggerId,
        adapter::{
            db::TriggerRepository,
            db_sqlite::{SQLiteTriggerRepository, TriggerRow},
        },
    },
};

pub struct TriggerService {
    postgres_repo: TriggerRepository,
    sqlite_repo: SQLiteTriggerRepository,
    sqlite_db: SQLiteDatabase,
    event_tx: EventEmitter<TriggerEvent>,
}

impl TriggerService {
    pub fn new(
        postgres_repo: TriggerRepository,
        sqlite_db: SQLiteDatabase,
        event_tx: EventEmitter<TriggerEvent>,
    ) -> anyhow::Result<Self> {
        let sqlite_repo = {
            let connection = sqlite_db.for_writing()?;
            SQLiteTriggerRepository::new(&connection)?
        };
        Ok(Self {
            postgres_repo,
            sqlite_repo,
            sqlite_db,
            event_tx,
        })
    }

    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let timestamp_us = t!(now).into_db().timestamp_micros();
        let correlation_id = TraceContext::current().correlation_id().map(|id| id.to_string());
        let trigger_json = serde_json::to_string(&trigger)?;
        self.postgres_repo.add_trigger(trigger).await?;

        self.sqlite_db
            .for_writing()
            .and_then(|connection| {
                self.sqlite_repo
                    .insert(&connection, &trigger_json, timestamp_us, correlation_id.as_deref())
            })
            .context("Trigger was saved to Postgres but could not be saved to SQLite")?;
        self.event_tx.send(TriggerEvent::TriggerAdded);
        Ok(())
    }

    pub fn get_all_triggers_active_anytime_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let start_us = range.start().into_db().timestamp_micros();
        let end_us = (*range.end()).min(t!(now)).into_db().timestamp_micros();
        let rows = self.read_rows(i64::MIN, end_us)?;
        Ok(rows
            .into_iter()
            .filter(|row| row.timestamp_us >= start_us || row.active_until_us.is_none_or(|until| until >= start_us))
            .filter_map(parse_trigger)
            .collect())
    }

    pub fn get_latest_triggers(&self) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let now = t!(now);
        let since = now - t!(48 hours);
        let rows = self.read_rows(since.into_db().timestamp_micros(), now.into_db().timestamp_micros())?;
        let mut seen_targets = HashSet::new();
        Ok(rows
            .into_iter()
            .filter_map(parse_trigger)
            .filter(|execution| seen_targets.insert(execution.target()))
            .map(|mut execution| {
                // Match the existing Postgres latest-trigger view, not the historical view.
                execution.active_until = execution.active_until.filter(|until| now <= *until);
                execution
            })
            .collect())
    }

    fn read_rows(&self, start_us: i64, end_us: i64) -> anyhow::Result<Vec<TriggerRow>> {
        let connection = self.sqlite_db.for_reading()?;
        self.sqlite_repo.created_between(&connection, start_us, end_us)
    }

    pub fn disable_triggers_before_except(
        &self,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        let before_us = before.into_db().timestamp_micros();
        let excluded_ids: HashSet<_> = excluded_ids.iter().map(UserTriggerId::value).collect();
        let mut connection = self.sqlite_db.for_writing()?;
        let transaction = connection.transaction()?;
        let ids = self.sqlite_repo.open_ids_created_before(&transaction, before_us)?;
        let mut count = 0;
        for id in ids.into_iter().filter(|id| !excluded_ids.contains(id)) {
            count += self.sqlite_repo.set_active_until(&transaction, id, before_us)?;
        }
        transaction.commit()?;
        Ok(count)
    }

    pub fn set_triggers_active_from_if_unset(&self, trigger_ids: &[UserTriggerId]) -> anyhow::Result<u64> {
        let from_us = t!(now).into_db().timestamp_micros();
        let mut connection = self.sqlite_db.for_writing()?;
        let transaction = connection.transaction()?;
        let mut count = 0;
        for id in trigger_ids {
            count += self
                .sqlite_repo
                .set_active_from_if_unset(&transaction, id.value(), from_us)?;
        }
        transaction.commit()?;
        Ok(count)
    }
}

fn parse_trigger(row: TriggerRow) -> Option<UserTriggerExecution> {
    fn parse_timestamp(micros: i64) -> anyhow::Result<DateTime> {
        chrono::DateTime::from_timestamp_micros(micros)
            .map(DateTime::from)
            .context("Invalid trigger timestamp")
    }

    let execution = (|| -> anyhow::Result<_> {
        Ok(UserTriggerExecution {
            id: row.id.into(),
            trigger: serde_json::from_str(&row.trigger)?,
            timestamp: parse_timestamp(row.timestamp_us)?,
            active_from: row.active_from_us.map(parse_timestamp).transpose()?,
            active_until: row.active_until_us.map(parse_timestamp).transpose()?,
            correlation_id: row.correlation_id,
        })
    })();
    match execution {
        Ok(execution) => Some(execution),
        Err(error) => {
            tracing::warn!(id = row.id, %error, "Invalid SQLite user_trigger row, ignoring");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::OnOffDevice;
    use infrastructure::EventBus;
    use rusqlite::params;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct TestFile(PathBuf);

    impl TestFile {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!("rusty-home-triggers-{}-{id}.sqlite", std::process::id())))
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn new_trigger_is_written_to_both_but_lifecycle_uses_sqlite_ids(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let file = TestFile::new();
        let db = SQLiteDatabase::open(&file.0)?;
        let bus = EventBus::new(8);
        let mut events = bus.subscribe();
        let service = TriggerService::new(TriggerRepository::new(pool.clone()), db.clone(), bus.emitter())?;
        let now = DateTime::from_static_iso("2026-03-10T12:00:00.123456Z");
        let start = now + t!(1 minutes);
        let stop = now + t!(2 minutes);
        let trigger = UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on: true,
        };
        // Force different ID spaces to catch accidental Postgres lifecycle updates or reads.
        db.for_writing()?.execute(
            "INSERT INTO user_trigger (id, trigger, timestamp_us) VALUES (500, ?1, ?2)",
            params![
                serde_json::to_string(&trigger)?,
                (now - t!(1 hours)).into_db().timestamp_micros()
            ],
        )?;

        now.eval_timeshifted(service.add_trigger(trigger)).await?;
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.recv()).await?;
        assert!(matches!(event, Some(TriggerEvent::TriggerAdded)));
        let latest = now.eval_timeshifted(async { service.get_latest_triggers() }).await?;
        assert_eq!(latest.len(), 1);
        let id = latest[0].id.clone();
        assert_eq!(id.value(), 501);
        assert_eq!(latest[0].timestamp, now);
        assert_eq!(
            start
                .eval_timeshifted(async { service.set_triggers_active_from_if_unset(std::slice::from_ref(&id)) })
                .await?,
            1
        );
        assert_eq!(
            stop.eval_timeshifted(async { service.set_triggers_active_from_if_unset(std::slice::from_ref(&id)) })
                .await?,
            0
        );
        assert_eq!(service.disable_triggers_before_except(stop, std::slice::from_ref(&id))?, 1);
        assert_eq!(service.disable_triggers_before_except(stop, &[])?, 1);
        assert_eq!(service.disable_triggers_before_except(stop + t!(1 minutes), &[])?, 0);

        let history = stop
            .eval_timeshifted(async { service.get_all_triggers_active_anytime_in_range(DateTimeRange::since(start)) })
            .await?;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, id);
        assert_eq!(history[0].active_from, Some(start));
        assert_eq!(history[0].active_until, Some(stop));
        assert_eq!(history[1].active_until, Some(stop));

        let rows: Vec<(i64, bool, bool)> =
            sqlx::query_as("SELECT id, active_from IS NULL, active_until IS NULL FROM user_trigger")
                .fetch_all(&pool)
                .await?;
        assert_eq!(rows, vec![(1, true, true)]);
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn a_failed_dual_write_does_not_make_a_trigger_visible_to_automation(
        pool: sqlx::PgPool,
    ) -> anyhow::Result<()> {
        let file = TestFile::new();
        let db = SQLiteDatabase::open(&file.0)?;
        let bus = EventBus::new(8);
        let mut events = bus.subscribe();
        let service = TriggerService::new(TriggerRepository::new(pool.clone()), db.clone(), bus.emitter())?;
        let trigger = UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on: true,
        };
        db.for_writing()?.execute_batch(
            "CREATE TRIGGER reject_insert BEFORE INSERT ON user_trigger BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )?;

        let result = service.add_trigger(trigger.clone()).await;
        let postgres_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM user_trigger")
            .fetch_one(&pool)
            .await?;
        assert!(result.is_err());
        assert_eq!(postgres_rows, 1);
        assert!(service.get_latest_triggers()?.is_empty());

        db.for_writing()?.execute_batch("DROP TRIGGER reject_insert;")?;
        pool.close().await;
        assert!(service.add_trigger(trigger).await.is_err());
        assert!(service.get_latest_triggers()?.is_empty());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), events.recv())
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn latest_and_historical_reads_keep_their_distinct_time_windows() -> anyhow::Result<()> {
        let file = TestFile::new();
        let db = SQLiteDatabase::open(&file.0)?;
        let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://localhost/not_used")?;
        let service = TriggerService::new(TriggerRepository::new(pool), db.clone(), EventBus::new(8).emitter())?;
        let now = DateTime::from_static_iso("2026-03-10T12:00:00Z");
        let start = now - t!(30 minutes);
        let expired = now - t!(10 minutes);
        let trigger = serde_json::to_string(&UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on: true,
        })?;
        {
            let connection = db.for_writing()?;
            for (id, timestamp, until, json) in [
                (1, now - t!(2 hours), Some(expired), trigger.as_str()),
                (2, now - t!(5 minutes), None, r#"{"type":"removed_trigger"}"#),
                (3, now + t!(1 minutes), None, trigger.as_str()),
                (4, now - t!(2 hours), Some(start - t!(1 minutes)), trigger.as_str()),
            ] {
                connection.execute(
                    "INSERT INTO user_trigger (id, trigger, timestamp_us, active_until_us) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        id,
                        json,
                        timestamp.into_db().timestamp_micros(),
                        until.map(|dt| dt.into_db().timestamp_micros())
                    ],
                )?;
            }
        }
        now.eval_timeshifted(async {
            let latest = service.get_latest_triggers()?;
            assert_eq!(latest.len(), 1);
            // Same target and timestamp: newest SQLite ID wins.
            assert_eq!(latest[0].id.value(), 4);
            assert_eq!(latest[0].active_until, None);
            let history =
                service.get_all_triggers_active_anytime_in_range(DateTimeRange::new(start, now + t!(1 hours)))?;
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].id.value(), 1);
            assert_eq!(history[0].active_until, Some(expired));
            Ok(())
        })
        .await
    }
}
