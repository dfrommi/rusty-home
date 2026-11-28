use std::collections::HashSet;

use anyhow::Context as _;
use rusqlite::{Connection, params};

use crate::{
    core::time::DateTime,
    trigger::{UserTriggerExecution, UserTriggerId, UserTriggerTarget},
};

struct TriggerRow {
    id: i64,
    trigger: String,
    timestamp_us: i64,
    active_from_us: Option<i64>,
    active_until_us: Option<i64>,
    correlation_id: Option<String>,
}

pub struct SQLiteTriggerRepository;

impl SQLiteTriggerRepository {
    pub fn new(connection: &Connection) -> anyhow::Result<Self> {
        connection.execute_batch(
            r#"CREATE TABLE IF NOT EXISTS user_trigger (
                id INTEGER PRIMARY KEY,
                trigger TEXT NOT NULL,
                timestamp_us INTEGER NOT NULL,
                active_from_us INTEGER,
                active_until_us INTEGER,
                correlation_id TEXT
            ) STRICT;
            CREATE INDEX IF NOT EXISTS user_trigger_timestamp_id_idx
                ON user_trigger (timestamp_us DESC, id DESC);"#,
        )?;
        Ok(Self)
    }

    pub fn insert(
        &self,
        connection: &Connection,
        trigger: &str,
        timestamp: DateTime,
        correlation_id: Option<&str>,
    ) -> anyhow::Result<()> {
        connection.execute(
            "INSERT INTO user_trigger (trigger, timestamp_us, correlation_id) VALUES (?1, ?2, ?3)",
            params![trigger, timestamp.unix_micros(), correlation_id],
        )?;
        Ok(())
    }

    pub fn created_between(
        &self,
        connection: &Connection,
        start: DateTime,
        end: DateTime,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        let mut statement = connection.prepare_cached(
            "SELECT id, trigger, timestamp_us, active_from_us, active_until_us, correlation_id
             FROM user_trigger
             WHERE timestamp_us BETWEEN ?1 AND ?2
             ORDER BY timestamp_us DESC, id DESC",
        )?;
        let rows = statement.query_map(params![start.unix_micros(), end.unix_micros()], read_trigger_row)?;
        parse_rows(rows)
    }

    pub fn latest_for_target(
        &self,
        connection: &Connection,
        target: &UserTriggerTarget,
        as_of: DateTime,
    ) -> anyhow::Result<Option<UserTriggerExecution>> {
        let mut statement = connection.prepare_cached(
            "SELECT id, trigger, timestamp_us, active_from_us, active_until_us, correlation_id
             FROM user_trigger
             WHERE timestamp_us <= ?1
             ORDER BY timestamp_us DESC, id DESC",
        )?;
        let rows = statement.query_map([as_of.unix_micros()], read_trigger_row)?;

        for row in rows {
            if let Some(execution) = parse_trigger(row?)
                && execution.target() == *target
            {
                return Ok(Some(execution));
            }
        }

        Ok(None)
    }
    pub fn disable_before_except(
        &self,
        connection: &Connection,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        let excluded_ids: HashSet<_> = excluded_ids.iter().map(UserTriggerId::value).collect();
        let mut statement = connection
            .prepare_cached("SELECT id FROM user_trigger WHERE active_until_us IS NULL AND timestamp_us < ?1")?;
        let rows = statement.query_map([before.unix_micros()], |row| row.get(0))?;
        let mut count = 0;
        for id in rows.collect::<rusqlite::Result<Vec<i64>>>()? {
            if !excluded_ids.contains(&id) {
                count += self.set_active_until(connection, id, before)?;
            }
        }
        Ok(count)
    }

    pub fn set_active_until(&self, connection: &Connection, id: i64, until: DateTime) -> anyhow::Result<u64> {
        let mut statement = connection.prepare_cached("UPDATE user_trigger SET active_until_us = ?2 WHERE id = ?1")?;
        Ok(statement.execute(params![id, until.unix_micros()])? as u64)
    }

    pub fn set_active_from_if_unset(
        &self,
        connection: &Connection,
        trigger_ids: &[UserTriggerId],
        from: DateTime,
    ) -> anyhow::Result<u64> {
        let mut statement = connection
            .prepare_cached("UPDATE user_trigger SET active_from_us = ?2 WHERE id = ?1 AND active_from_us IS NULL")?;
        let mut count = 0;
        for id in trigger_ids {
            count += statement.execute(params![id.value(), from.unix_micros()])? as u64;
        }
        Ok(count)
    }
}

fn read_trigger_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TriggerRow> {
    Ok(TriggerRow {
        id: row.get(0)?,
        trigger: row.get(1)?,
        timestamp_us: row.get(2)?,
        active_from_us: row.get(3)?,
        active_until_us: row.get(4)?,
        correlation_id: row.get(5)?,
    })
}

fn parse_rows(rows: impl Iterator<Item = rusqlite::Result<TriggerRow>>) -> anyhow::Result<Vec<UserTriggerExecution>> {
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(parse_trigger)
        .collect())
}

fn parse_trigger(row: TriggerRow) -> Option<UserTriggerExecution> {
    fn parse_timestamp(micros: i64) -> anyhow::Result<DateTime> {
        DateTime::try_from_unix_micros(micros).context("Invalid SQLite user trigger timestamp")
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
