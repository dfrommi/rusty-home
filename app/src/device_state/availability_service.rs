use std::collections::{HashMap, HashSet};

use infrastructure::SQLiteDatabase;

use crate::{
    core::time::DateTime,
    device_state::{
        DeviceAvailability, DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityRecord,
        DeviceAvailabilityStatus,
        adapter::db::{DeviceAvailabilityRepository, SQLiteDeviceAvailabilityRepository},
    },
    t,
};

pub struct DeviceAvailabilityService {
    postgres_repo: DeviceAvailabilityRepository,
    sqlite_repo: SQLiteDeviceAvailabilityRepository,
    sqlite_db: SQLiteDatabase,
    config: DeviceAvailabilityConfig,
}

impl DeviceAvailabilityService {
    pub fn new(
        postgres_repo: DeviceAvailabilityRepository,
        sqlite_db: SQLiteDatabase,
        config: DeviceAvailabilityConfig,
    ) -> anyhow::Result<Self> {
        let sqlite_repo = {
            let connection = sqlite_db.for_writing()?;
            SQLiteDeviceAvailabilityRepository::new(&connection)?
        };

        Ok(Self {
            postgres_repo,
            sqlite_repo,
            sqlite_db,
            config,
        })
    }

    pub async fn initialize(&self, items: HashSet<DeviceAvailabilityItem>) -> anyhow::Result<()> {
        self.postgres_repo.sync_item_availability(&items).await?;

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

    pub async fn handle_update(&self, availability: DeviceAvailability) {
        let record = DeviceAvailabilityRecord {
            item: availability.item,
            last_seen: availability.last_seen,
            marked_offline: availability.marked_offline,
            entry_updated: t!(now),
            disabled: false,
        };

        if let Err(error) = self
            .postgres_repo
            .update_device_availability(
                &record.item.item,
                &record.item.source,
                &record.last_seen,
                record.marked_offline,
            )
            .await
        {
            tracing::error!("Error updating device availability for {}: {:?}", record.item.item, error);
            return;
        }

        let sqlite_result = self
            .sqlite_db
            .for_writing()
            .and_then(|connection| self.sqlite_repo.save(&connection, &record));
        if let Err(error) = sqlite_result {
            tracing::error!(
                "Error syncing device availability with SQLite for {}: {:?}",
                record.item.item,
                error
            );
        }

        tracing::info!(
            "Device availability updated for {}: marked_offline={}",
            record.item.item,
            record.marked_offline
        );
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        let records = match self
            .sqlite_db
            .for_reading()
            .and_then(|connection| self.sqlite_repo.get_all(&connection))
        {
            Ok(records) => records,
            Err(error) => {
                tracing::error!(
                    "Error reading device availability from SQLite, falling back to PostgreSQL: {:?}",
                    error
                );
                return self.postgres_repo.get_item_availabilities().await;
            }
        };
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
