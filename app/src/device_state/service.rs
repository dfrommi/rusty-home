use std::collections::{HashMap, HashSet};

use infrastructure::EventEmitter;

use crate::{
    core::{
        time::{DateTime, DateTimeRange},
        timeseries::DataPoint,
    },
    device_state::{
        DeviceAvailability, DeviceAvailabilityItem, DeviceAvailabilityStatus, DeviceStateEvent, DeviceStateId,
        DeviceStateValue,
        adapter::db::{CachedDeviceStateRepository, DeviceAvailabilityRepository, SaveOutcome},
    },
};

pub struct DeviceStateService {
    state_repo: CachedDeviceStateRepository,
    availability_repo: DeviceAvailabilityRepository,
    event_tx: EventEmitter<DeviceStateEvent>,
}

impl DeviceStateService {
    pub fn new(
        state_repo: CachedDeviceStateRepository,
        availability_repo: DeviceAvailabilityRepository,
        event_tx: EventEmitter<DeviceStateEvent>,
    ) -> Self {
        Self {
            state_repo,
            availability_repo,
            event_tx,
        }
    }

    pub async fn initialize_availability(&self, items: HashSet<DeviceAvailabilityItem>) -> anyhow::Result<()> {
        self.availability_repo.sync_item_availability(&items).await
    }

    pub async fn handle_state_update(&self, dp: DataPoint<DeviceStateValue>) {
        if DateTime::is_shifted() {
            tracing::warn!("Received device state update with shifted DateTime: {:?}, ignoring", dp);
            return;
        }

        let id = DeviceStateId::from(&dp.value);

        let outcome = match self.state_repo.save(dp.clone()).await {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::error!("Error saving device state for {:?}: {:?}", id, e);
                return;
            }
        };

        if outcome == SaveOutcome::Stale {
            tracing::debug!("Ignoring stale device state update for {:?}: {:?}", id, dp);
            return;
        }

        tracing::info!("Device state update received ({:?}): {:?}", outcome, &dp.value);

        self.event_tx.send(DeviceStateEvent::Updated(dp.clone()));
        if outcome == SaveOutcome::Inserted {
            // Only for new values, to preserve the time when the value became active.
            self.event_tx.send(DeviceStateEvent::Changed(dp));
        }
    }

    pub async fn handle_availability_update(&self, avail: DeviceAvailability) {
        match self
            .availability_repo
            .update_device_availability(&avail.item.item, &avail.item.source, &avail.last_seen, avail.marked_offline)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Device availability updated for {}: marked_offline={}",
                    avail.item.item,
                    avail.marked_offline
                );
            }
            Err(e) => {
                tracing::error!("Error updating device availability for {}: {:?}", avail.item.item, e);
            }
        }
    }

    pub async fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        let mut res = HashMap::new();

        for id in DeviceStateId::variants() {
            match self.state_repo.get_latest_for_device(&id).await {
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

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        self.state_repo.get_all_data_points_in_range_ts_asc(range).await
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        self.availability_repo.get_item_availabilities().await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use infrastructure::EventBus;
    use sqlx::PgPool;

    use crate::{
        core::{timeseries::DataPoint, unit::DegreeCelsius},
        device_state::{
            DeviceAvailabilityConfig, DeviceStateEvent, DeviceStateValue, Temperature,
            adapter::db::{CachedDeviceStateRepository, DeviceAvailabilityRepository, PostgresDeviceStateRepository},
        },
        t,
    };

    use super::DeviceStateService;

    #[sqlx::test(migrations = "../migrations")]
    async fn stale_updates_do_not_emit_events_but_duplicates_still_emit_updated(pool: PgPool) -> anyhow::Result<()> {
        let event_bus = EventBus::new(8);
        let mut events = event_bus.subscribe();
        let service = DeviceStateService::new(
            CachedDeviceStateRepository::new(PostgresDeviceStateRepository::new(pool.clone())),
            DeviceAvailabilityRepository::new(
                pool,
                DeviceAvailabilityConfig {
                    default_offline_after: t!(1 hours),
                    overrides: vec![],
                },
            ),
            event_bus.emitter(),
        );
        let latest = DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(22.0)),
            t!(now),
        );

        service.handle_state_update(latest.clone()).await;
        assert!(matches!(events.recv().await, Some(DeviceStateEvent::Updated(_))));
        assert!(matches!(events.recv().await, Some(DeviceStateEvent::Changed(_))));

        service.handle_state_update(latest).await;
        assert!(matches!(events.recv().await, Some(DeviceStateEvent::Updated(_))));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), events.recv())
                .await
                .is_err()
        );

        service
            .handle_state_update(DataPoint::new(
                DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0)),
                t!(1 minutes ago),
            ))
            .await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), events.recv())
                .await
                .is_err()
        );

        Ok(())
    }
}
