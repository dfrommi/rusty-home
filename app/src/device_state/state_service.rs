use std::collections::HashMap;

use anyhow::Context as _;
use infrastructure::{EventEmitter, SQLiteDatabase};
use moka::{ops::compute::Op, sync::Cache};

use crate::{
    core::{
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{
        DeviceStateEvent, DeviceStateId, DeviceStateValue,
        adapter::db::{SQLiteDeviceStateRepository, SaveOutcome},
    },
};

type LatestValues = Cache<DeviceStateId, Option<DataPoint<DeviceStateValue>>>;

pub struct DeviceStateService {
    sqlite_repo: SQLiteDeviceStateRepository,
    sqlite_db: SQLiteDatabase,
    event_tx: EventEmitter<DeviceStateEvent>,
    latest_values: LatestValues,
}

impl DeviceStateService {
    pub fn new(
        sqlite_repo: SQLiteDeviceStateRepository,
        sqlite_db: SQLiteDatabase,
        event_tx: EventEmitter<DeviceStateEvent>,
    ) -> Self {
        Self {
            sqlite_repo,
            sqlite_db,
            event_tx,
            latest_values: Cache::builder().max_capacity(100_000).build(),
        }
    }

    pub(super) fn handle_state_update(&self, dp: DataPoint<DeviceStateValue>) -> anyhow::Result<()> {
        if DateTime::is_shifted() {
            tracing::warn!("Received device state update with shifted DateTime: {:?}, ignoring", dp);
            return Ok(());
        }

        let id = DeviceStateId::from(&dp.value);
        let outcome = {
            let connection = self
                .sqlite_db
                .for_writing()
                .with_context(|| format!("Error opening SQLite for device state {id:?}"))?;
            let current_dp = self.sqlite_repo.get_latest_for_device_for_save(&connection, &id)?;
            let outcome = save_outcome(&dp, current_dp.as_ref());

            if let Some(current_dp) = current_dp.as_ref() {
                match outcome {
                    SaveOutcome::Duplicate => tracing::debug!(
                        "Duplicate device state update for {:?}: value {:?} is equal to or within the minimum storage delta of {:?}",
                        id,
                        dp.value,
                        current_dp.value
                    ),
                    SaveOutcome::Stale => tracing::debug!(
                        "Stale device state update for {:?}: current timestamp {:?} is newer than update timestamp {:?}",
                        id,
                        current_dp.timestamp,
                        dp.timestamp
                    ),
                    SaveOutcome::Inserted => tracing::debug!("Inserting new device state for {:?}: {:?}", id, dp),
                }
            } else {
                tracing::debug!("No current device state for {:?}, inserting new value: {:?}", id, dp);
            }
            if outcome == SaveOutcome::Inserted {
                self.sqlite_repo
                    .save(&connection, dp.clone())
                    .with_context(|| format!("Error saving device state for {id:?} to SQLite"))?;
                tracing::info!("Saved device state to SQLite for {:?}: {:?}", id, &dp.value);
            }

            outcome
        };

        // Invalidate only after saving, and wait on the cache without holding the SQLite connection.
        if outcome == SaveOutcome::Inserted {
            invalidate_latest(&self.latest_values, id);
        }

        if outcome == SaveOutcome::Stale {
            tracing::debug!("Ignoring stale device state update for {:?}: {:?}", id, dp);
            return Ok(());
        }

        tracing::info!("Device state update received ({:?}): {:?}", outcome, &dp.value);
        self.event_tx.send(DeviceStateEvent::Updated(dp.clone()));

        if outcome == SaveOutcome::Inserted {
            tracing::info!(
                event = "device_state_changed",
                device_type = id.ext_id().type_name(),
                device_id = id.ext_id().variant_name(),
                value = f64::from(&dp.value),
                "Device state changed for {:?}: {:?}",
                id,
                &dp.value
            );

            // Only for new values, to preserve the time when the value became active.
            self.event_tx.send(DeviceStateEvent::Changed(dp));
        }
        Ok(())
    }
    pub fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        let mut res = HashMap::new();
        for id in DeviceStateId::variants() {
            match self.get_latest_for_device(&id) {
                Ok(Some(dp)) => {
                    res.insert(id, dp);
                }
                Ok(None) => {
                    // Skip silently — device has no current value
                }
                Err(e) => {
                    tracing::warn!("Error getting latest device state for {:?}: {:?}", id, e);
                }
            }
        }
        Ok(res)
    }

    pub fn get_latest_for_device(&self, id: &DeviceStateId) -> anyhow::Result<Option<DataPoint<DeviceStateValue>>> {
        let read = || {
            let connection = self.sqlite_db.for_reading()?;
            self.sqlite_repo.get_latest_for_device(&connection, id)
        };
        if DateTime::is_shifted() {
            return read();
        }

        read_latest_cached(&self.latest_values, *id, read)
            .with_context(|| format!("Error getting latest device state for {id:?}"))
    }

    pub fn get_state_since(
        &self,
        id: &DeviceStateId,
        since: DateTime,
    ) -> anyhow::Result<Option<DataFrame<DeviceStateValue>>> {
        let connection = self.sqlite_db.for_reading()?;
        self.sqlite_repo.get_data_frame_since(&connection, id, since)
    }
}

fn save_outcome(incoming: &DataPoint<DeviceStateValue>, current: Option<&DataPoint<DeviceStateValue>>) -> SaveOutcome {
    let Some(current) = current else {
        return SaveOutcome::Inserted;
    };

    if incoming.timestamp < current.timestamp {
        return SaveOutcome::Stale;
    }

    let incoming_value = f64::from(&incoming.value);
    let current_value = f64::from(&current.value);
    let minimum_delta = incoming.value.minimum_storage_delta().unwrap_or(0.0);
    if incoming_value == current_value || (incoming_value - current_value).abs() < minimum_delta {
        SaveOutcome::Duplicate
    } else {
        SaveOutcome::Inserted
    }
}

fn read_latest_cached(
    cache: &LatestValues,
    id: DeviceStateId,
    read: impl FnOnce() -> anyhow::Result<Option<DataPoint<DeviceStateValue>>>,
) -> anyhow::Result<Option<DataPoint<DeviceStateValue>>> {
    if let Some(value) = cache.get(&id) {
        return Ok(value);
    }

    let result = cache.entry(id).and_try_compute_with(|current| -> anyhow::Result<_> {
        if current.is_some() {
            Ok(Op::Nop)
        } else {
            Ok(Op::Put(read()?))
        }
    })?;
    Ok(result.into_entry().and_then(|entry| entry.into_value()))
}

fn invalidate_latest(cache: &LatestValues, id: DeviceStateId) {
    // Use the same per-key operation as a fill, so removal waits for an in-progress fill.
    cache.entry(id).and_compute_with(|_| Op::Remove);
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{atomic::AtomicU64, mpsc},
        thread,
        time::Duration,
    };

    use infrastructure::{EventBus, SQLiteDatabase};

    use super::*;
    use crate::{
        core::{time::DateTime, timeseries::DataPoint, unit::DegreeCelsius},
        device_state::{CompletionTime, DeviceStateValue, Presence, Temperature},
        t,
    };
    #[test]
    fn save_outcome_filters_values_inside_minimum_storage_delta() {
        let now = DateTime::now();
        let completion_time = CompletionTime::WashingMachineEstimated;
        let current = DataPoint::new(DeviceStateValue::CompletionTime(completion_time, now + t!(1 hours)), now);
        let incoming = DataPoint::new(
            DeviceStateValue::CompletionTime(completion_time, now + t!(61 minutes)),
            now + t!(1 seconds),
        );

        assert_eq!(save_outcome(&incoming, Some(&current)), SaveOutcome::Duplicate);
    }

    #[test]
    fn save_outcome_inserts_values_at_the_minimum_storage_delta() {
        let now = DateTime::now();
        let completion_time = CompletionTime::WashingMachineEstimated;
        let current = DataPoint::new(DeviceStateValue::CompletionTime(completion_time, now + t!(1 hours)), now);
        let incoming = DataPoint::new(
            DeviceStateValue::CompletionTime(completion_time, now + t!(62 minutes)),
            now + t!(1 seconds),
        );

        assert_eq!(save_outcome(&incoming, Some(&current)), SaveOutcome::Inserted);
    }

    #[test]
    fn save_outcome_marks_same_value_stale_update_as_stale() {
        let current = DataPoint::new(
            DeviceStateValue::Presence(Presence::LivingRoomCouch, true),
            DateTime::from_static_iso("2026-03-10T12:00:00.123456Z"),
        );
        let incoming = DataPoint::new(current.value.clone(), DateTime::from_static_iso("2026-03-10T12:00:00.123455Z"));

        assert_eq!(save_outcome(&incoming, Some(&current)), SaveOutcome::Stale);
    }

    #[test]
    fn save_outcome_uses_full_timestamp_precision() {
        let current = DataPoint::new(
            DeviceStateValue::Presence(Presence::LivingRoomCouch, false),
            DateTime::from_static_iso("2026-03-10T12:00:00.123456Z"),
        );
        let incoming = DataPoint::new(
            DeviceStateValue::Presence(Presence::LivingRoomCouch, true),
            DateTime::from_static_iso("2026-03-10T12:00:00.123455Z"),
        );

        assert_eq!(save_outcome(&incoming, Some(&current)), SaveOutcome::Stale);
    }

    #[test]
    fn save_outcome_matches_values_after_database_conversion() {
        let current = DataPoint::new(
            DeviceStateValue::CompletionTime(
                CompletionTime::WashingMachineEstimated,
                DateTime::from_static_iso("2026-03-10T12:00:00.123100Z"),
            ),
            DateTime::from_static_iso("2026-03-10T12:00:00Z"),
        );
        let incoming = DataPoint::new(
            DeviceStateValue::CompletionTime(
                CompletionTime::WashingMachineEstimated,
                DateTime::from_static_iso("2026-03-10T12:00:00.123900Z"),
            ),
            DateTime::from_static_iso("2026-03-10T12:00:01Z"),
        );

        assert_eq!(save_outcome(&incoming, Some(&current)), SaveOutcome::Duplicate);
    }

    #[tokio::test]
    async fn sqlite_write_failure_is_returned_without_emitting_state_events() -> anyhow::Result<()> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rusty-home-device-state-write-failure-{}-{id}.sqlite",
            std::process::id()
        ));
        fs::File::create_new(&path)?;
        let database_file = TestFile(path);
        let database = SQLiteDatabase::open(&database_file.0)?;
        let repository = {
            let connection = database.for_writing()?;
            SQLiteDeviceStateRepository::new(&connection)?
        };
        let bus = EventBus::new(8);
        let mut events = bus.subscribe();
        let service = DeviceStateService::new(repository, database.clone(), bus.emitter());
        database.for_writing()?.execute_batch(
            "CREATE TRIGGER reject_insert BEFORE INSERT ON device_state BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )?;

        let update = DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.0)),
            DateTime::now(),
        );
        assert!(service.handle_state_update(update).is_err());
        assert!(
            service
                .get_latest_for_device(&DeviceStateId::Temperature(Temperature::LivingRoom))?
                .is_none()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(10), events.recv())
                .await
                .is_err()
        );
        Ok(())
    }

    struct TestFile(PathBuf);

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn invalidation_discards_a_cache_fill_that_started_before_the_write() -> anyhow::Result<()> {
        let cache = LatestValues::new(8);
        let id = DeviceStateId::Temperature(Temperature::LivingRoom);
        let (old_value_read, wait_for_read) = mpsc::channel();
        let (resume, continue_fill) = mpsc::channel();
        let (invalidation_started, wait_for_start) = mpsc::channel();
        let (invalidated, wait_for_invalidation) = mpsc::channel();
        let timeout = Duration::from_secs(2);

        thread::scope(|scope| -> anyhow::Result<()> {
            let cache = &cache;
            let fill = scope.spawn(move || {
                read_latest_cached(cache, id, || {
                    old_value_read.send(())?;
                    continue_fill.recv_timeout(timeout)?;
                    Ok(None)
                })
            });
            wait_for_read.recv_timeout(timeout)?;

            let writer = scope.spawn(move || -> anyhow::Result<()> {
                invalidation_started.send(())?;
                invalidate_latest(cache, id);
                invalidated.send(())?;
                Ok(())
            });
            wait_for_start.recv_timeout(timeout)?;
            let before_fill = wait_for_invalidation.recv_timeout(Duration::from_millis(25));
            resume.send(())?;
            assert_eq!(before_fill, Err(mpsc::RecvTimeoutError::Timeout));
            assert!(fill.join().expect("cache fill thread panicked")?.is_none());
            writer.join().expect("cache invalidation thread panicked")?;
            Ok(())
        })?;

        let new_value = DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0));
        let latest = read_latest_cached(&cache, id, || Ok(Some(DataPoint::new(new_value.clone(), DateTime::now()))))?
            .context("Expected the cache to read the new value after invalidation")?;
        assert_eq!(latest.value, new_value);
        Ok(())
    }
}
