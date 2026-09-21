use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use infrastructure::EventEmitter;
use moka::future::Cache;

use crate::{
    core::{
        time::{DateTime, DateTimeRange},
        timeseries::DataPoint,
    },
    device_state::{
        DeviceAvailability, DeviceAvailabilityItem, DeviceAvailabilityStatus, DeviceStateEvent, DeviceStateId,
        DeviceStateValue, adapter::db::DeviceStateRepository,
    },
};

pub struct DeviceStateService {
    repo: DeviceStateRepository,
    event_tx: EventEmitter<DeviceStateEvent>,
    current_cache: Cache<DeviceStateId, DataPoint<DeviceStateValue>>,
    availability_items: OnceLock<HashSet<DeviceAvailabilityItem>>,
}

impl DeviceStateService {
    pub fn new(repo: DeviceStateRepository, event_tx: EventEmitter<DeviceStateEvent>) -> Self {
        let current_cache = Cache::builder().max_capacity(10_000).build();

        Self {
            repo,
            event_tx,
            current_cache,
            availability_items: OnceLock::new(),
        }
    }

    pub fn initialize_availability(&self, items: HashSet<DeviceAvailabilityItem>) -> anyhow::Result<()> {
        self.availability_items
            .set(items)
            .map_err(|_| anyhow::anyhow!("Device availability was already initialized"))
    }

    pub async fn handle_state_update(&self, dp: DataPoint<DeviceStateValue>) {
        if DateTime::is_shifted() {
            tracing::warn!("Received device state update with shifted DateTime: {:?}, ignoring", dp);
            return;
        }

        let id = DeviceStateId::from(&dp.value);

        let changed = match self.repo.save(dp.clone()).await {
            Ok(changed) => changed,
            Err(e) => {
                tracing::error!("Error saving device state for {:?}: {:?}", id, e);
                return;
            }
        };

        tracing::info!("Device state update received (changed = {}): {:?}", changed, &dp.value);

        self.event_tx.send(DeviceStateEvent::Updated(dp.clone()));
        if changed {
            //Only when changed to preserve timestamps (new one not to be used unless value is new)
            self.current_cache.insert(id, dp.clone()).await;
            self.event_tx.send(DeviceStateEvent::Changed(dp.clone()));
        }
    }

    pub async fn handle_availability_update(&self, avail: DeviceAvailability) {
        match self
            .repo
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
            match self.get_latest_for_device(&id).await {
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

    async fn get_latest_for_device(&self, id: &DeviceStateId) -> anyhow::Result<Option<DataPoint<DeviceStateValue>>> {
        if DateTime::is_shifted() {
            //TODO uncached bootstapping leads to a lot of db hits, improve this
            return self.repo.get_latest_for_device(id).await;
        }

        if let Some(dp) = self.current_cache.get(id).await {
            return Ok(Some(dp));
        }

        tracing::debug!("Cache miss for device state {:?}, fetching from repo", id);
        let Some(dp) = self.repo.get_latest_for_device(id).await? else {
            return Ok(None);
        };
        self.current_cache.insert(*id, dp.clone()).await;
        Ok(Some(dp))
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        self.repo.get_all_data_points_in_range_ts_asc(range).await
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        self.repo.get_item_availabilities().await
    }
}
