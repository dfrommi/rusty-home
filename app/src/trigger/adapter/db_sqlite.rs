use rusqlite::{Connection, params};

pub struct TriggerRow {
    pub id: i64,
    pub trigger: String,
    pub timestamp_us: i64,
    pub active_from_us: Option<i64>,
    pub active_until_us: Option<i64>,
    pub correlation_id: Option<String>,
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
        timestamp_us: i64,
        correlation_id: Option<&str>,
    ) -> anyhow::Result<()> {
        connection.execute(
            "INSERT INTO user_trigger (trigger, timestamp_us, correlation_id) VALUES (?1, ?2, ?3)",
            params![trigger, timestamp_us, correlation_id],
        )?;
        Ok(())
    }

    pub fn created_between(
        &self,
        connection: &Connection,
        start_us: i64,
        end_us: i64,
    ) -> anyhow::Result<Vec<TriggerRow>> {
        let mut statement = connection.prepare_cached(
            "SELECT id, trigger, timestamp_us, active_from_us, active_until_us, correlation_id
             FROM user_trigger
             WHERE timestamp_us BETWEEN ?1 AND ?2
             ORDER BY timestamp_us DESC, id DESC",
        )?;
        let rows = statement.query_map(params![start_us, end_us], |row| {
            Ok(TriggerRow {
                id: row.get(0)?,
                trigger: row.get(1)?,
                timestamp_us: row.get(2)?,
                active_from_us: row.get(3)?,
                active_until_us: row.get(4)?,
                correlation_id: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn open_ids_created_before(&self, connection: &Connection, before_us: i64) -> anyhow::Result<Vec<i64>> {
        let mut statement = connection
            .prepare_cached("SELECT id FROM user_trigger WHERE active_until_us IS NULL AND timestamp_us < ?1")?;
        let rows = statement.query_map([before_us], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_active_until(&self, connection: &Connection, id: i64, until_us: i64) -> anyhow::Result<u64> {
        let mut statement = connection.prepare_cached("UPDATE user_trigger SET active_until_us = ?2 WHERE id = ?1")?;
        Ok(statement.execute(params![id, until_us])? as u64)
    }

    pub fn set_active_from_if_unset(&self, connection: &Connection, id: i64, from_us: i64) -> anyhow::Result<u64> {
        let mut statement = connection
            .prepare_cached("UPDATE user_trigger SET active_from_us = ?2 WHERE id = ?1 AND active_from_us IS NULL")?;
        Ok(statement.execute(params![id, from_us])? as u64)
    }
}
