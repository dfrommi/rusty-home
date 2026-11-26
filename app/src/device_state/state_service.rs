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
                let current_dp = self.sqlite_repo.get_latest_for_device(&connection, &id)?;
                let sqlite_outcome = match current_dp {
                    None => {
                        tracing::debug!("SQLite: No current device state for {:?}, inserting new value: {:?}", id, dp);
                        SaveOutcome::Inserted
                    }
                    Some(current_dp) if dp.value == current_dp.value => {
                        tracing::debug!(
                            "SQLite: Duplicate device state update for {:?}: value {:?} is the same as the current value {:?}",
                            id,
                            dp.value,
                            current_dp.value
                        );
                        SaveOutcome::Duplicate
                    }
                    //TODO better internal timestamp handling without unneccessary high precision, for
                    //examlple truncate to millis
                    Some(current_dp) if dp.timestamp.unix_millis() < current_dp.timestamp.unix_millis() => {
                        tracing::debug!(
                            "SQLite: Stale device state update for {:?}: current timestamp {:?} is newer than update timestamp {:?}",
                            id,
                            current_dp.timestamp,
                            dp.timestamp
                        );
                        SaveOutcome::Stale
                    }
                    _ => {
                        tracing::debug!("SQLite: Inserting new device state for {:?}: {:?}", id, dp);
                        SaveOutcome::Inserted
                    }
                };

                if outcome == SaveOutcome::Inserted {
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

    use crate::{core::unit::DegreeCelsius, device_state::Temperature};

    use super::*;

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
