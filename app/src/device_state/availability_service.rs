use std::collections::{HashMap, HashSet};

use infrastructure::SQLiteDatabase;

use crate::{
    core::time::DateTime,
    device_state::{
        DeviceAvailability, DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityRecord,
        DeviceAvailabilityStatus, adapter::db::SQLiteDeviceAvailabilityRepository,
    },
    t,
};

pub struct DeviceAvailabilityService {
    sqlite_repo: SQLiteDeviceAvailabilityRepository,
    sqlite_db: SQLiteDatabase,
    config: DeviceAvailabilityConfig,
}

impl DeviceAvailabilityService {
    pub fn new(
        sqlite_repo: SQLiteDeviceAvailabilityRepository,
        sqlite_db: SQLiteDatabase,
        config: DeviceAvailabilityConfig,
    ) -> Self {
        Self {
            sqlite_repo,
            sqlite_db,
            config,
        }
    }

    pub fn initialize(&self, items: HashSet<DeviceAvailabilityItem>) -> anyhow::Result<()> {
        self.config.warn_unknown_items(&items);

        let records = {
            let connection = self.sqlite_db.for_reading()?;
            self.sqlite_repo.get_all(&connection)?
        };
        let records = reconcile_availability_records(records, &items, t!(now));

        let connection = self.sqlite_db.for_writing()?;
        for record in &records {
            self.sqlite_repo.save(&connection, record)?;
        }

        Ok(())
    }

    pub fn handle_update(&self, availability: DeviceAvailability) -> anyhow::Result<()> {
        let record = DeviceAvailabilityRecord {
            item: availability.item,
            last_seen: availability.last_seen,
            marked_offline: availability.marked_offline,
            entry_updated: t!(now),
            disabled: false,
        };

        let connection = self.sqlite_db.for_writing()?;
        self.sqlite_repo.save(&connection, &record)?;

        tracing::info!(
            "Device availability updated for {}: marked_offline={}",
            record.item.item,
            record.marked_offline
        );
        Ok(())
    }

    pub fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        let records = self
            .sqlite_db
            .for_reading()
            .and_then(|connection| self.sqlite_repo.get_all(&connection))?;
        let now = t!(now);

        Ok(records
            .into_iter()
            .map(|record| availability_status(record, &self.config, now))
            .collect())
    }
}

fn reconcile_availability_records(
    records: Vec<DeviceAvailabilityRecord>,
    configured_items: &HashSet<DeviceAvailabilityItem>,
    now: DateTime,
) -> Vec<DeviceAvailabilityRecord> {
    let mut records: HashMap<_, _> = records
        .into_iter()
        .map(|record| (record.item.clone(), record))
        .collect();

    for record in records.values_mut() {
        record.disabled = !configured_items.contains(&record.item);
    }

    for item in configured_items {
        records.entry(item.clone()).or_insert_with(|| DeviceAvailabilityRecord {
            item: item.clone(),
            last_seen: now,
            marked_offline: true,
            entry_updated: now,
            disabled: false,
        });
    }

    records.into_values().collect()
}

fn availability_status(
    record: DeviceAvailabilityRecord,
    config: &DeviceAvailabilityConfig,
    now: DateTime,
) -> DeviceAvailabilityStatus {
    let offline_after = if record.disabled {
        &config.default_offline_after
    } else {
        config.offline_after(&record.item.source, &record.item.item)
    };
    let last_seen_ago = std::cmp::max(now.elapsed_since(record.last_seen), now.elapsed_since(record.entry_updated));
    let is_offline = record.marked_offline || last_seen_ago > offline_after.clone();

    DeviceAvailabilityStatus {
        source: record.item.source,
        item: record.item.item,
        last_seen_ago,
        is_offline,
        disabled: record.disabled,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use infrastructure::SQLiteDatabase;

    use super::*;
    use crate::{device_state::DeviceAvailabilityOverride, t};
    fn record(item: DeviceAvailabilityItem, timestamp: DateTime, disabled: bool) -> DeviceAvailabilityRecord {
        DeviceAvailabilityRecord {
            item,
            last_seen: timestamp,
            marked_offline: false,
            entry_updated: timestamp,
            disabled,
        }
    }

    #[test]
    fn sqlite_write_failure_is_returned_to_the_caller() -> anyhow::Result<()> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rusty-home-availability-write-failure-{}-{id}.sqlite",
            std::process::id()
        ));
        fs::File::create_new(&path)?;
        let file = TestFile(path);
        let database = SQLiteDatabase::open(&file.0)?;
        let repository = {
            let connection = database.for_writing()?;
            SQLiteDeviceAvailabilityRepository::new(&connection)?
        };
        let service = DeviceAvailabilityService::new(
            repository,
            database.clone(),
            DeviceAvailabilityConfig {
                default_offline_after: t!(1 hours),
                overrides: vec![],
            },
        );
        database.for_writing()?.execute_batch(
            "CREATE TRIGGER reject_insert BEFORE INSERT ON item_availability BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )?;

        let availability = DeviceAvailability {
            item: DeviceAvailabilityItem::new("HA", "sensor.test"),
            last_seen: DateTime::now(),
            marked_offline: false,
        };
        assert!(service.handle_update(availability).is_err());
        Ok(())
    }

    struct TestFile(PathBuf);

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn reconciliation_reenables_configured_items_and_disables_removed_items() {
        let now = DateTime::from_static_iso("2025-01-01T10:00:00Z");
        let existing = DeviceAvailabilityItem::new("HA", "existing");
        let removed = DeviceAvailabilityItem::new("HA", "removed");
        let new = DeviceAvailabilityItem::new("HA", "new");
        let old_timestamp = now - t!(10 minutes);
        let records = reconcile_availability_records(
            vec![
                record(existing.clone(), old_timestamp, true),
                record(removed.clone(), old_timestamp, false),
            ],
            &HashSet::from([existing.clone(), new.clone()]),
            now,
        );
        let records: HashMap<_, _> = records
            .into_iter()
            .map(|record| (record.item.clone(), record))
            .collect();

        let existing_record = &records[&existing];
        assert!(!existing_record.disabled);
        assert_eq!(existing_record.last_seen, old_timestamp);
        assert_eq!(existing_record.entry_updated, old_timestamp);

        let new_record = &records[&new];
        assert!(!new_record.disabled);
        assert!(new_record.marked_offline);
        assert_eq!(new_record.last_seen, now);
        assert_eq!(new_record.entry_updated, now);

        let removed_record = &records[&removed];
        assert!(removed_record.disabled);
        assert_eq!(removed_record.last_seen, old_timestamp);
    }

    #[test]
    fn status_uses_overrides_only_for_enabled_items_and_ages_both_timestamps() {
        let now = DateTime::from_static_iso("2025-01-01T10:00:00Z");
        let config = DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![DeviceAvailabilityOverride {
                source: "HA".to_string(),
                item: "slow_sensor".to_string(),
                offline_after: t!(3 hours),
            }],
        };

        let override_record = record(DeviceAvailabilityItem::new("HA", "slow_sensor"), now - t!(2 hours), false);
        assert!(!availability_status(override_record, &config, now).is_offline);

        let disabled_record = record(DeviceAvailabilityItem::new("HA", "slow_sensor"), now - t!(2 hours), true);
        let status = availability_status(disabled_record, &config, now);
        assert!(status.is_offline);
        assert!(status.disabled);

        let mut explicitly_offline = record(DeviceAvailabilityItem::new("HA", "slow_sensor"), now, false);
        explicitly_offline.marked_offline = true;
        assert!(availability_status(explicitly_offline, &config, now).is_offline);

        let mut old_entry = record(DeviceAvailabilityItem::new("HA", "slow_sensor"), now - t!(5 minutes), false);
        old_entry.entry_updated = now - t!(4 hours);
        assert!(availability_status(old_entry, &config, now).is_offline);
    }
}
