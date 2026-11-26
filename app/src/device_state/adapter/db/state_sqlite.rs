use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::{
    core::{
        id::ExternalId,
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateId, DeviceStateValue},
};

use super::state::from_f64_value;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS device_state_tag (
    id INTEGER PRIMARY KEY,
    channel TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (channel, name)
) STRICT;
CREATE TABLE IF NOT EXISTS device_state (
    id INTEGER PRIMARY KEY,
    tag_id INTEGER NOT NULL REFERENCES device_state_tag(id),
    value REAL NOT NULL,
    timestamp_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS device_state_tag_timestamp_id_idx
    ON device_state (tag_id, timestamp_ms DESC, id DESC);
"#;

#[derive(Clone)]
pub struct SQLiteDeviceStateRepository {
    tag_ids: HashMap<DeviceStateId, i64>,
}

impl SQLiteDeviceStateRepository {
    /// Initialize the device-state schema and tag IDs with the supplied connection.
    pub fn new(connection: &Connection) -> Result<Self> {
        connection.execute_batch(SCHEMA)?;
        let tag_ids = load_tag_ids(connection)?;

        Ok(Self { tag_ids })
    }

    pub fn save(&self, connection: &Connection, dp: DataPoint<DeviceStateValue>) -> Result<()> {
        let id = DeviceStateId::from(&dp.value);
        let tag_id = self.tag_id(id)?;
        connection.execute(
            "INSERT INTO device_state (tag_id, value, timestamp_ms) VALUES (?1, ?2, ?3)",
            params![tag_id, f64::from(&dp.value), dp.timestamp.unix_millis()],
        )?;

        Ok(())
    }

    pub fn get_latest_for_device(
        &self,
        connection: &Connection,
        id: &DeviceStateId,
    ) -> Result<Option<DataPoint<DeviceStateValue>>> {
        let tag_id = self.tag_id(*id)?;
        let mut statement = connection.prepare_cached(
            r#"
            SELECT value, timestamp_ms
            FROM device_state
            WHERE tag_id = ?1 AND timestamp_ms <= ?2
            ORDER BY timestamp_ms DESC, id DESC
            LIMIT 1
            "#,
        )?;

        let mut rows = statement.query(params![tag_id, DateTime::now().unix_millis()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(DataPoint {
            value: from_f64_value(*id, row.get(0)?),
            timestamp: DateTime::from_unix_millis(row.get(1)?),
        }))
    }

    pub fn get_data_frame_since(
        &self,
        connection: &Connection,
        id: &DeviceStateId,
        since: DateTime,
    ) -> Result<Option<DataFrame<DeviceStateValue>>> {
        let tag_id = self.tag_id(*id)?;
        let now = DateTime::now();
        let mut statement = connection.prepare_cached(
            r#"
            SELECT value, timestamp_ms
            FROM device_state
            WHERE tag_id = ?1 AND timestamp_ms <= ?3
              AND timestamp_ms >= COALESCE((
                SELECT MAX(timestamp_ms)
                FROM device_state
                WHERE tag_id = ?1 AND timestamp_ms <= MIN(?2, ?3)
              ), ?2)
            ORDER BY timestamp_ms ASC, id ASC
            "#,
        )?;

        let rows = statement.query_map(params![tag_id, since.unix_millis(), now.unix_millis()], |row| {
            Ok(DataPoint {
                value: from_f64_value(*id, row.get(0)?),
                timestamp: DateTime::from_unix_millis(row.get(1)?),
            })
        })?;

        Ok(DataFrame::new(rows.collect::<rusqlite::Result<Vec<_>>>()?).non_empty())
    }

    fn tag_id(&self, id: DeviceStateId) -> Result<i64> {
        self.tag_ids
            .get(&id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No SQLite tag configured for device state {:?}", id))
    }
}

fn load_tag_ids(connection: &Connection) -> Result<HashMap<DeviceStateId, i64>> {
    let existing_tags = {
        let mut statement = connection.prepare("SELECT channel, name FROM device_state_tag")?;
        let rows = statement.query_map([], |row| {
            Ok(ExternalId::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<HashSet<_>, _>>()?
    };

    for id in DeviceStateId::variants() {
        let external_id = id.ext_id();
        if existing_tags.contains(&external_id) {
            continue;
        }

        connection.execute(
            "INSERT INTO device_state_tag (channel, name) VALUES (?1, ?2)",
            params![external_id.type_name(), external_id.variant_name()],
        )?;
    }

    let mut tag_ids = HashMap::new();
    let mut statement = connection.prepare("SELECT id, channel, name FROM device_state_tag")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;

    for row in rows {
        let (tag_id, channel, name) = row?;
        let external_id = ExternalId::new(channel, name);
        if let Ok(id) = DeviceStateId::try_from(external_id)
            && tag_ids.insert(id, tag_id).is_some()
        {
            anyhow::bail!("Duplicate SQLite device-state tag for {:?}", id);
        }
    }

    Ok(tag_ids)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{fs, path::PathBuf, sync::atomic::AtomicU64};

    use infrastructure::SQLiteDatabase;
    use rusqlite::Connection;

    use crate::{
        core::{time::DateTime, timeseries::DataPoint, unit::DegreeCelsius},
        device_state::{DeviceStateId, DeviceStateValue, Presence, Temperature},
        t,
    };

    use super::SQLiteDeviceStateRepository;

    static NEXT_TEST_FILE_ID: AtomicU64 = AtomicU64::new(0);

    fn test_file() -> PathBuf {
        let id = NEXT_TEST_FILE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("rusty-home-device-state-sqlite-{}-{id}.db", std::process::id()))
    }

    #[test]
    fn creates_strict_schema_and_round_trips_device_state_data() -> anyhow::Result<()> {
        let path = test_file();
        fs::File::create_new(&path)?;
        let database = SQLiteDatabase::open(&path)?;
        let repository = {
            let connection = database.for_writing()?;
            SQLiteDeviceStateRepository::new(&connection)?
        };
        let now = t!(now);
        let living_room = DeviceStateId::Temperature(Temperature::LivingRoom);

        {
            let connection = database.for_writing()?;
            let strict_tables: i64 = connection.query_row(
                "SELECT count(*) FROM pragma_table_list WHERE name IN ('device_state_tag', 'device_state') AND strict = 1",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(strict_tables, 2);

            repository.save(
                &connection,
                DataPoint::new(
                    DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.0)),
                    now - t!(30 minutes),
                ),
            )?;
            repository.save(
                &connection,
                DataPoint::new(
                    DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0)),
                    now - t!(20 minutes),
                ),
            )?;
            repository.save(
                &connection,
                DataPoint::new(
                    DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.0)),
                    now - t!(10 minutes),
                ),
            )?;
        }

        let (latest, data_frame) = {
            let connection = database.for_reading()?;
            let latest = repository
                .get_latest_for_device(&connection, &living_room)?
                .expect("latest device state not found");
            let data_frame = repository
                .get_data_frame_since(&connection, &living_room, now - t!(25 minutes))?
                .expect("device state history not found");
            (latest, data_frame)
        };

        assert_eq!(
            latest.value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0))
        );
        assert_eq!(data_frame.len(), 2);
        assert_eq!(
            data_frame.first().unwrap().value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.0))
        );
        assert_eq!(
            data_frame.last().unwrap().value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0))
        );

        drop(database);
        fs::remove_file(path)?;

        Ok(())
    }

    #[tokio::test]
    async fn history_starts_at_an_exact_match_and_hides_rows_after_shifted_now() -> anyhow::Result<()> {
        let connection = Connection::open_in_memory()?;
        let repository = SQLiteDeviceStateRepository::new(&connection)?;
        let cutoff = DateTime::from_static_iso("2026-03-10T12:00:00Z");
        let now = cutoff + t!(5 minutes);
        let couch = DeviceStateId::Presence(Presence::LivingRoomCouch);
        let bedroom = DeviceStateId::Temperature(Temperature::Bedroom);
        for (timestamp, value) in [
            (
                cutoff - t!(1 seconds),
                DeviceStateValue::Presence(Presence::LivingRoomCouch, true),
            ),
            (cutoff, DeviceStateValue::Presence(Presence::LivingRoomCouch, false)),
            (now + t!(1 seconds), DeviceStateValue::Presence(Presence::LivingRoomCouch, true)),
            (
                now + t!(1 seconds),
                DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.0)),
            ),
        ] {
            repository.save(&connection, DataPoint::new(value, timestamp))?;
        }

        now.eval_timeshifted(async {
            let history = repository
                .get_data_frame_since(&connection, &couch, cutoff)?
                .expect("couch history not found");
            assert_eq!(history.len(), 1);
            assert!(!history.any(|dp| dp.value == DeviceStateValue::Presence(Presence::LivingRoomCouch, true)));
            assert!(
                repository
                    .get_data_frame_since(&connection, &bedroom, now + t!(2 seconds))?
                    .is_none()
            );
            Ok(())
        })
        .await
    }
}
