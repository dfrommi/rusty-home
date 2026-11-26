use std::collections::HashMap;

use infrastructure::{EventEmitter, SQLiteDatabase};
use moka::sync::Cache;

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

pub struct DeviceStateService {
    state_repo: PostgresDeviceStateRepository,
    sqlite_repo: SQLiteDeviceStateRepository,
    sqlite_db: SQLiteDatabase,
    event_tx: EventEmitter<DeviceStateEvent>,
    latest_values: Cache<DeviceStateId, Option<DataPoint<DeviceStateValue>>>,
}

impl DeviceStateService {
    pub fn new(
        state_repo: PostgresDeviceStateRepository,
        sqlite_db: SQLiteDatabase,
        event_tx: EventEmitter<DeviceStateEvent>,
    ) -> anyhow::Result<Self> {
        let sqlite_repo = {
            let connection = sqlite_db.for_writing()?;
            SQLiteDeviceStateRepository::new(&connection)?
        };

        Ok(Self {
            state_repo,
            sqlite_repo,
            sqlite_db,
            event_tx,
            latest_values: Cache::builder().max_capacity(100_000).build(),
        })
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

            self.latest_values.invalidate(&id);

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
        if DateTime::is_shifted() {
            let connection = self.sqlite_db.for_reading()?;
            return self.sqlite_repo.get_latest_for_device(&connection, id);
        }

        self.latest_values
            .try_get_with(*id, || {
                let connection = self.sqlite_db.for_reading()?;
                self.sqlite_repo.get_latest_for_device(&connection, id)
            })
            .map_err(|e| anyhow::anyhow!("Error getting latest device state for {:?}: {:?}", id, e))
    }

    pub fn get_state_since(&self, id: &DeviceStateId, since: DateTime) -> anyhow::Result<DataFrame<DeviceStateValue>> {
        let connection = self.sqlite_db.for_reading()?;
        self.sqlite_repo.get_data_frame_since(&connection, id, since)
    }
}
