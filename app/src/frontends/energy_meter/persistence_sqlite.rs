use anyhow::Context as _;
use rusqlite::{Connection, params};

use crate::core::time::DateTime;

pub struct RawEnergyReading {
    pub reading_type: String,
    pub name: String,
    pub value: f64,
    pub timestamp: DateTime,
    pub year_end: bool,
}

pub struct SQLiteEnergyReadingRepository;

impl SQLiteEnergyReadingRepository {
    pub fn new(connection: &Connection) -> anyhow::Result<Self> {
        connection.execute_batch(
            r#"CREATE TABLE IF NOT EXISTS energy_reading (
                id INTEGER PRIMARY KEY,
                type TEXT NOT NULL,
                name TEXT NOT NULL,
                value REAL NOT NULL,
                timestamp_us INTEGER NOT NULL,
                year_end INTEGER NOT NULL CHECK (year_end IN (0, 1))
            ) STRICT;"#,
        )?;
        Ok(Self)
    }

    pub fn insert(&self, connection: &Connection, reading: &RawEnergyReading) -> anyhow::Result<()> {
        connection.execute(
            "INSERT INTO energy_reading (type, name, value, timestamp_us, year_end) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                reading.reading_type,
                reading.name,
                reading.value,
                reading.timestamp.unix_micros(),
                reading.year_end
            ],
        )?;
        Ok(())
    }

    pub fn get_all(&self, connection: &Connection) -> anyhow::Result<Vec<RawEnergyReading>> {
        let mut statement = connection.prepare_cached(
            "SELECT type, name, value, timestamp_us, year_end FROM energy_reading ORDER BY timestamp_us DESC, id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (reading_type, name, value, timestamp_us, year_end) = row?;
            let timestamp = DateTime::try_from_unix_micros(timestamp_us).context("Invalid energy reading timestamp")?;
            Ok(RawEnergyReading {
                reading_type,
                name,
                value,
                timestamp,
                year_end,
            })
        })
        .collect()
    }
}
