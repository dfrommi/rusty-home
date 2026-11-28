use std::sync::Arc;

use infrastructure::{EventEmitter, SQLiteDatabase};
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::DataPoint;

mod http_server;
mod persistence_sqlite;
mod service;

use persistence_sqlite::SQLiteEnergyReadingRepository;
use service::EnergyReadingService;

#[derive(Clone)]
pub struct EnergyMeter {
    service: Arc<EnergyReadingService>,
}

impl EnergyMeter {
    pub fn new(sqlite_db: SQLiteDatabase, tx: EventEmitter<DataPoint<EnergyReading>>) -> anyhow::Result<Self> {
        let sqlite_repo = {
            let connection = sqlite_db.for_writing()?;
            SQLiteEnergyReadingRepository::new(&connection)?
        };
        let service = EnergyReadingService::new(sqlite_repo, sqlite_db, tx);
        Ok(Self {
            service: Arc::new(service),
        })
    }

    pub fn new_web_service(&self) -> actix_web::Scope {
        http_server::new_actix_web_scope(self.service.clone())
    }

    pub fn publish_initial_readings(&self) -> anyhow::Result<()> {
        self.service.publish_initial_readings()
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergyMeterTarget {
    Heating(Radiator),
}

#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
}

impl EnergyReading {
    pub fn target(&self) -> EnergyMeterTarget {
        match self {
            Self::Heating(item, _) => EnergyMeterTarget::Heating(*item),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use infrastructure::EventBus;
    use rusqlite::params;

    use crate::core::time::DateTime;

    use super::*;

    pub struct TestDatabaseFile(PathBuf);

    impl Drop for TestDatabaseFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    pub fn test_database() -> anyhow::Result<(TestDatabaseFile, SQLiteDatabase)> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rusty-home-energy-{}-{id}.sqlite", std::process::id()));
        fs::File::create_new(&path)?;
        let file = TestDatabaseFile(path);
        let db = SQLiteDatabase::open(&file.0)?;
        Ok((file, db))
    }

    pub fn seed_heating_readings(db: &SQLiteDatabase, rows: &[(&str, f64, &str, bool)]) -> anyhow::Result<()> {
        let connection = db.for_writing()?;
        for (name, value, timestamp, year_end) in rows {
            connection.execute(
                "INSERT INTO energy_reading (type, name, value, timestamp_us, year_end) VALUES ('heating', ?1, ?2, ?3, ?4)",
                params![name, value, DateTime::from_iso(timestamp)?.unix_micros(), year_end],
            )?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn startup_publishes_latest_totals_from_sqlite_without_adding_readings() -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let meter = EnergyMeter::new(db.clone(), bus.emitter())?;
        seed_heating_readings(
            &db,
            &[
                ("bedroom", 100.0, "2024-12-31T23:00:00Z", true),
                ("bedroom", 12.5, "2025-01-10T12:00:00Z", false),
                ("kitchen", 900.0, "2024-12-31T23:00:00Z", true),
                ("kitchen", 50.0, "2025-01-11T12:00:00Z", false),
                ("removed_radiator", 77.0, "2025-01-11T12:00:00Z", false),
            ],
        )?;
        meter.publish_initial_readings()?;

        let mut totals = HashMap::new();
        for _ in 0..2 {
            let reading = tokio::time::timeout(std::time::Duration::from_secs(1), readings.recv())
                .await?
                .expect("energy meter source closed");
            totals.insert(reading.value.target(), reading);
        }
        for (radiator, expected, timestamp) in [
            (Radiator::Bedroom, 112.5, "2025-01-10T12:00:00Z"),
            (Radiator::Kitchen, 950.0, "2025-01-11T12:00:00Z"),
        ] {
            let reading = totals
                .get(&EnergyMeterTarget::Heating(radiator))
                .expect("missing initial total");
            match reading.value {
                EnergyReading::Heating(_, value) => assert_eq!(value, expected),
            }
            assert_eq!(reading.timestamp, DateTime::from_static_iso(timestamp));
        }

        let count: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!(count, 5);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), readings.recv())
                .await
                .is_err()
        );

        Ok(())
    }
}
