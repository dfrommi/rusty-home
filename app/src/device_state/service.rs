use std::collections::HashMap;

use moka::future::Cache;
use tokio::sync::broadcast;

use crate::{
    core::{time::DateTimeRange, timeseries::DataPoint},
    device_state::{
        DeviceAvailability, DeviceStateEvent, DeviceStateId, DeviceStateValue, OfflineItem,
        adapter::db::DeviceStateRepository,
    },
};

pub struct DeviceStateService {
    repo: DeviceStateRepository,
    event_tx: broadcast::Sender<DeviceStateEvent>,
    current_cache: Cache<DeviceStateId, DataPoint<DeviceStateValue>>,
}

impl DeviceStateService {
    pub fn new(repo: DeviceStateRepository, event_tx: broadcast::Sender<DeviceStateEvent>) -> Self {
        let current_cache = Cache::builder().max_capacity(10_000).build();

        Self {
            repo,
            event_tx,
            current_cache,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DeviceStateEvent> {
        self.event_tx.subscribe()
    }

    pub async fn handle_state_update(&self, dp: DataPoint<DeviceStateValue>) {
        let id = DeviceStateId::from(&dp.value);

        let changed = match self.repo.save(dp.clone()).await {
            Ok(changed) => changed,
            Err(e) => {
                tracing::error!("Error saving device state for {:?}: {:?}", id, e);
                return;
            }
        };

        tracing::info!("Device state update received (changed = {}): {:?}", changed, &dp.value);

        self.current_cache.insert(id, dp.clone()).await;

        //publish event
        if let Err(e) = self.event_tx.send(DeviceStateEvent::Updated(dp.clone())) {
            tracing::error!("Error sending device state updated event for {:?}: {}", id, e);
        }
        if changed && let Err(e) = self.event_tx.send(DeviceStateEvent::Changed(dp.clone())) {
            tracing::error!("Error sending device state changed event for {:?}: {}", id, e);
        }
    }

    pub async fn handle_availability_update(&self, avail: DeviceAvailability) {
        match self
            .repo
            .update_device_availability(&avail.device_id, &avail.source, &avail.last_seen, avail.marked_offline)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Device availability updated for {}: marked_offline={}",
                    avail.device_id,
                    avail.marked_offline
                );
            }
            Err(e) => {
                tracing::error!("Error updating device availability for {}: {:?}", avail.device_id, e);
            }
        }
    }

    pub async fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        //self.repo.get_latest_for_all_devices().await
        let mut res = HashMap::new();

        for id in DeviceStateId::variants() {
            match self.get_latest_for_device(&id).await {
                Ok(dp) => {
                    res.insert(id, dp);
                }
                Err(e) => {
                    tracing::warn!("Error getting latest device state for {:?}: {:?}", id, e);
                }
            }
        }

        Ok(res)
    }

    pub async fn get_latest_for_device(&self, id: &DeviceStateId) -> anyhow::Result<DataPoint<DeviceStateValue>> {
        self.current_cache
            .try_get_with(*id, async {
                tracing::debug!("Cache miss for device state {:?}, fetching from repo", id);
                self.repo.get_latest_for_device(id).await
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        self.repo.get_all_data_points_in_range(range).await
    }

    pub async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        self.repo.get_offline_items().await
    }
}
