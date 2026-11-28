use std::collections::HashSet;

use anyhow::Context as _;
use infrastructure::{EventEmitter, SQLiteDatabase, TraceContext};

use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
    trigger::{
        TriggerEvent, UserTrigger, UserTriggerExecution, UserTriggerId, adapter::db_sqlite::SQLiteTriggerRepository,
    },
};

pub struct TriggerService {
    sqlite_repo: SQLiteTriggerRepository,
    sqlite_db: SQLiteDatabase,
    event_tx: EventEmitter<TriggerEvent>,
}

impl TriggerService {
    pub fn new(
        sqlite_repo: SQLiteTriggerRepository,
        sqlite_db: SQLiteDatabase,
        event_tx: EventEmitter<TriggerEvent>,
    ) -> Self {
        Self {
            sqlite_repo,
            sqlite_db,
            event_tx,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        let timestamp = t!(now);
        let correlation_id = TraceContext::current().correlation_id().map(|id| id.to_string());
        let trigger_json = serde_json::to_string(&trigger)?;
        self.sqlite_db
            .for_writing()
            .and_then(|connection| {
                self.sqlite_repo
                    .insert(&connection, &trigger_json, timestamp, correlation_id.as_deref())
            })
            .context("Could not save trigger to SQLite")?;
        self.event_tx.send(TriggerEvent::TriggerAdded);
        Ok(())
    }

    pub fn get_all_triggers_active_anytime_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let start = *range.start();
        let end = (*range.end()).min(t!(now));
        let connection = self.sqlite_db.for_reading()?;
        self.sqlite_repo.active_anytime_in_range(&connection, start, end)
    }

    pub fn get_latest_triggers(&self) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let now = t!(now);
        let since = now - t!(48 hours);
        let connection = self.sqlite_db.for_reading()?;
        let triggers = self.sqlite_repo.created_between(&connection, since, now)?;
        let mut seen_targets = HashSet::new();
        Ok(triggers
            .into_iter()
            .filter(|execution| seen_targets.insert(execution.target()))
            .map(|mut execution| {
                execution.active_until = execution.active_until.filter(|until| now <= *until);
                execution
            })
            .collect())
    }

    pub fn disable_triggers_before_except(
        &self,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        let mut connection = self.sqlite_db.for_writing()?;
        let transaction = connection.transaction()?;
        let count = self
            .sqlite_repo
            .disable_before_except(&transaction, before, excluded_ids)?;
        transaction.commit()?;
        Ok(count)
    }

    pub fn set_triggers_active_from_if_unset(&self, trigger_ids: &[UserTriggerId]) -> anyhow::Result<u64> {
        let from = t!(now);
        let mut connection = self.sqlite_db.for_writing()?;
        let transaction = connection.transaction()?;
        let count = self
            .sqlite_repo
            .set_active_from_if_unset(&transaction, trigger_ids, from)?;
        transaction.commit()?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::{OnOffDevice, TriggerModule};
    use rusqlite::params;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct TestFile(PathBuf);

    impl TestFile {
        fn new() -> anyhow::Result<Self> {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("rusty-home-triggers-{}-{id}.sqlite", std::process::id()));
            fs::File::create_new(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[tokio::test]
    async fn trigger_is_saved_and_lifecycle_uses_sqlite_ids() -> anyhow::Result<()> {
        let file = TestFile::new()?;
        let db = SQLiteDatabase::open(&file.0)?;
        let module = TriggerModule::new(db.clone())?;
        let mut events = module.subscribe();
        let service = &module.service;
        let now = DateTime::from_static_iso("2026-03-10T12:00:00.123456Z");
        let start = now + t!(1 minutes);
        let stop = now + t!(2 minutes);
        let trigger = UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on: true,
        };
        db.for_writing()?.execute(
            "INSERT INTO user_trigger (id, trigger, timestamp_us) VALUES (500, ?1, ?2)",
            params![serde_json::to_string(&trigger)?, (now - t!(1 hours)).unix_micros()],
        )?;

        now.eval_timeshifted(async { service.add_trigger(trigger) }).await?;
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

        let rows: Vec<(i64, bool, bool)> = db
            .for_reading()?
            .prepare("SELECT id, active_from_us IS NULL, active_until_us IS NULL FROM user_trigger ORDER BY id")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        assert_eq!(rows, vec![(500, true, false), (501, false, false)]);
        Ok(())
    }
    #[tokio::test]
    async fn failed_sqlite_insert_does_not_make_trigger_visible_or_emit_event() -> anyhow::Result<()> {
        let file = TestFile::new()?;
        let db = SQLiteDatabase::open(&file.0)?;
        let module = TriggerModule::new(db.clone())?;
        let mut events = module.subscribe();
        let service = &module.service;
        let trigger = UserTrigger::DevicePower {
            device: OnOffDevice::InfraredHeater,
            on: true,
        };
        db.for_writing()?.execute_batch(
            "CREATE TRIGGER reject_insert BEFORE INSERT ON user_trigger BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )?;

        assert!(service.add_trigger(trigger.clone()).is_err());
        assert!(service.get_latest_triggers()?.is_empty());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), events.recv())
                .await
                .is_err()
        );

        db.for_writing()?.execute_batch("DROP TRIGGER reject_insert;")?;
        service.add_trigger(trigger)?;
        assert!(matches!(events.recv().await, Some(TriggerEvent::TriggerAdded)));
        assert_eq!(service.get_latest_triggers()?.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn latest_and_historical_reads_keep_their_distinct_time_windows() -> anyhow::Result<()> {
        let file = TestFile::new()?;
        let db = SQLiteDatabase::open(&file.0)?;
        let module = TriggerModule::new(db.clone())?;
        let service = &module.service;
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
                    params![id, json, timestamp.unix_micros(), until.map(|dt| dt.unix_micros())],
                )?;
            }
        }
        now.eval_timeshifted(async {
            let latest = service.get_latest_triggers()?;
            assert_eq!(latest.len(), 1);
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
