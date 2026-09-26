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
        adapter::db::{CachedDeviceStateRepository, DeviceAvailabilityRepository},
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

        let changed = match self.state_repo.save(dp.clone()).await {
            Ok(changed) => changed,
            Err(e) => {
                tracing::error!("Error saving device state for {:?}: {:?}", id, e);
                return;
            }
        };

        tracing::info!("Device state update received (changed = {}): {:?}", changed, &dp.value);

        self.event_tx.send(DeviceStateEvent::Updated(dp.clone()));
        if changed {
            // Only when changed to preserve timestamps (new one not to be used unless value is new).
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
