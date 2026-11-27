use anyhow::{Context as _, Result};
use rusqlite::{Connection, params};

use crate::{
    core::time::DateTime,
    device_state::{DeviceAvailabilityItem, DeviceAvailabilityRecord},
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS item_availability (
    source TEXT NOT NULL,
    item TEXT NOT NULL,
    last_seen_us INTEGER NOT NULL,
    marked_offline INTEGER NOT NULL CHECK (marked_offline IN (0, 1)),
    entry_updated_us INTEGER NOT NULL,
    disabled INTEGER NOT NULL CHECK (disabled IN (0, 1)),
    PRIMARY KEY (source, item)
) STRICT;
"#;

#[derive(Clone)]
pub struct SQLiteDeviceAvailabilityRepository;

impl SQLiteDeviceAvailabilityRepository {
    pub fn new(connection: &Connection) -> Result<Self> {
        connection.execute_batch(SCHEMA)?;
        Ok(Self)
    }

    pub fn get_all(&self, connection: &Connection) -> Result<Vec<DeviceAvailabilityRecord>> {
        let mut statement = connection.prepare_cached(
            r#"SELECT source, item, last_seen_us, marked_offline, entry_updated_us, disabled
               FROM item_availability"#,
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (source, item, last_seen_us, marked_offline, entry_updated_us, disabled) = row?;
            let last_seen = DateTime::try_from_unix_micros(last_seen_us)
                .context("Invalid SQLite availability last_seen timestamp")?;
            let entry_updated = DateTime::try_from_unix_micros(entry_updated_us)
                .context("Invalid SQLite availability entry_updated timestamp")?;
            Ok(DeviceAvailabilityRecord {
                item: DeviceAvailabilityItem::new(source, item),
                last_seen,
                marked_offline,
                entry_updated,
                disabled,
            })
        })
        .collect()
    }

    pub fn save(&self, connection: &Connection, record: &DeviceAvailabilityRecord) -> Result<()> {
        connection.execute(
            r#"INSERT INTO item_availability
               (source, item, last_seen_us, marked_offline, entry_updated_us, disabled)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT (source, item) DO UPDATE SET
                   last_seen_us = excluded.last_seen_us,
                   marked_offline = excluded.marked_offline,
                   entry_updated_us = excluded.entry_updated_us,
                   disabled = excluded.disabled"#,
            params![
                record.item.source,
                record.item.item,
                record.last_seen.unix_micros(),
                record.marked_offline,
                record.entry_updated.unix_micros(),
                record.disabled,
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{fs, path::PathBuf, sync::atomic::AtomicU64};

    use infrastructure::SQLiteDatabase;

    use crate::{
        core::time::DateTime,
        device_state::{DeviceAvailabilityItem, DeviceAvailabilityRecord},
    };

    use super::SQLiteDeviceAvailabilityRepository;

    static NEXT_TEST_FILE_ID: AtomicU64 = AtomicU64::new(0);

    fn test_file() -> PathBuf {
        let id = NEXT_TEST_FILE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("rusty-home-availability-sqlite-{}-{id}.db", std::process::id()))
    }

    #[test]
    fn invalid_timestamps_return_an_error() -> anyhow::Result<()> {
        let connection = rusqlite::Connection::open_in_memory()?;
        let repository = SQLiteDeviceAvailabilityRepository::new(&connection)?;
        connection.execute(
            "INSERT INTO item_availability VALUES ('HA', 'sensor.invalid', ?1, 0, ?1, 0)",
            [i64::MAX],
        )?;

        assert!(repository.get_all(&connection).is_err());
        Ok(())
    }

    #[test]
    fn stores_and_updates_availability_records() -> anyhow::Result<()> {
        let path = test_file();
        fs::File::create_new(&path)?;
        let database = SQLiteDatabase::open(&path)?;
        let repository = {
            let connection = database.for_writing()?;
            SQLiteDeviceAvailabilityRepository::new(&connection)?
        };
        let item = DeviceAvailabilityItem::new("HA", "sensor.temperature");
        let mut record = DeviceAvailabilityRecord {
            item: item.clone(),
            last_seen: DateTime::from_static_iso("2025-01-01T10:00:00.123456Z"),
            marked_offline: false,
            entry_updated: DateTime::from_static_iso("2025-01-01T10:01:00.654321Z"),
            disabled: false,
        };

        {
            let connection = database.for_writing()?;
            let strict: bool = connection.query_row(
                "SELECT strict FROM pragma_table_list WHERE name = 'item_availability'",
                [],
                |row| row.get(0),
            )?;
            assert!(strict);
            repository.save(&connection, &record)?;

            record.marked_offline = true;
            record.disabled = true;
            repository.save(&connection, &record)?;
        }

        let records = {
            let connection = database.for_reading()?;
            repository.get_all(&connection)?
        };
        assert_eq!(records, vec![record]);

        drop(database);
        fs::remove_file(path)?;
        Ok(())
    }
}
