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
        adapter::db::{PostgresDeviceStateRepository, SQLiteDeviceStateRepository, SaveOutcome},
    },
};

type LatestValues = Cache<DeviceStateId, Option<DataPoint<DeviceStateValue>>>;

pub struct DeviceStateService {
    state_repo: PostgresDeviceStateRepository,
    sqlite_repo: SQLiteDeviceStateRepository,
    sqlite_db: SQLiteDatabase,
    event_tx: EventEmitter<DeviceStateEvent>,
    latest_values: LatestValues,
}

impl DeviceStateService {
    pub fn new(
        state_repo: PostgresDeviceStateRepository,
        sqlite_repo: SQLiteDeviceStateRepository,
        sqlite_db: SQLiteDatabase,
        event_tx: EventEmitter<DeviceStateEvent>,
    ) -> Self {
        Self {
            state_repo,
            sqlite_repo,
            sqlite_db,
            event_tx,
            latest_values: Cache::builder().max_capacity(100_000).build(),
        }
    }

    pub(super) async fn handle_state_update(&self, dp: DataPoint<DeviceStateValue>) {
        if DateTime::is_shifted() {
            tracing::warn!("Received device state update with shifted DateTime: {:?}, ignoring", dp);
            return;
        }

        let id = DeviceStateId::from(&dp.value);
        let outcome = match self.state_repo.save(dp.clone()).await {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::error!("Error saving device state for {:?}: {:?}", id, error);
                return;
            }
        };

        // Keep SQLite's read/check/write flow under one lock. Do not hold this synchronous lock
        // across the Postgres await above.
        let sqlite_outcome = match self.sqlite_db.for_writing() {
            Ok(connection) => (|| -> anyhow::Result<SaveOutcome> {
                let current_dp = self.sqlite_repo.get_latest_for_device_for_save(&connection, &id)?;
                let sqlite_outcome = save_outcome(&dp, current_dp.as_ref());

                if let Some(current_dp) = current_dp.as_ref() {
                    match sqlite_outcome {
                        SaveOutcome::Duplicate => tracing::debug!(
                            "SQLite: Duplicate device state update for {:?}: value {:?} is equal to or within the minimum storage delta of {:?}",
                            id,
                            dp.value,
                            current_dp.value
                        ),
                        SaveOutcome::Stale => tracing::debug!(
                            "SQLite: Stale device state update for {:?}: current timestamp {:?} is newer than update timestamp {:?}",
                            id,
                            current_dp.timestamp,
                            dp.timestamp
                        ),
                        SaveOutcome::Inserted => {
                            tracing::debug!("SQLite: Inserting new device state for {:?}: {:?}", id, dp);
                        }
                    }
                } else {
                    tracing::debug!("SQLite: No current device state for {:?}, inserting new value: {:?}", id, dp);
                }
                if sqlite_outcome == SaveOutcome::Inserted {
                    self.sqlite_repo.save(&connection, dp.clone())?;
                    tracing::info!("Saved device state to SQLite for {:?}: {:?}", id, &dp.value);
                }

                Ok(sqlite_outcome)
            })(),
            Err(error) => Err(error),
        };

        // Invalidate only after saving, and wait on the cache without holding the SQLite connection.
        if outcome == SaveOutcome::Inserted && sqlite_outcome.is_ok() {
            invalidate_latest(&self.latest_values, id);
        }

        match sqlite_outcome {
            Ok(sqlite_outcome) if sqlite_outcome != outcome => tracing::error!(
                "Inconsistent save outcome for device state update of {:?}, trusting Postgres. SQLite: {}; Postgres {}",
                id,
                sqlite_outcome,
                outcome
            ),
            Ok(_) => {}
            Err(error) => tracing::error!("Error syncing device state with SQLite for {:?}: {:?}", id, error),
        }

        if outcome == SaveOutcome::Stale {
            tracing::debug!("Ignoring stale device state update for {:?}: {:?}", id, dp);
            return;
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
    use std::{sync::mpsc, thread, time::Duration};

    use crate::{
        core::{time::DateTime, timeseries::DataPoint, unit::DegreeCelsius},
        device_state::{CompletionTime, DeviceStateValue, Presence, Temperature},
        t,
    };

    use super::*;
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
