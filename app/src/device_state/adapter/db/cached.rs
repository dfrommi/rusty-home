use anyhow::Result;
use moka::future::Cache;

use crate::{
    core::{
        time::{DateTime, DateTimeRange},
        timeseries::DataPoint,
    },
    device_state::{DeviceStateId, DeviceStateValue},
};

use super::PostgresDeviceStateRepository;

pub struct CachedDeviceStateRepository {
    postgres: PostgresDeviceStateRepository,
    current_cache: Cache<DeviceStateId, DataPoint<DeviceStateValue>>,
}

impl CachedDeviceStateRepository {
    pub fn new(postgres: PostgresDeviceStateRepository) -> Self {
        Self {
            postgres,
            current_cache: Cache::builder().max_capacity(10_000).build(),
        }
    }

    pub async fn save(&self, dp: DataPoint<DeviceStateValue>) -> Result<bool> {
        let id = DeviceStateId::from(&dp.value);
        let changed = self.postgres.save(dp.clone()).await?;

        if changed {
            self.current_cache.insert(id, dp).await;
        }

        Ok(changed)
    }

    pub async fn get_latest_for_device(&self, id: &DeviceStateId) -> Result<Option<DataPoint<DeviceStateValue>>> {
        if DateTime::is_shifted() {
            return self.postgres.get_latest_for_device(id).await;
        }

        if let Some(dp) = self.current_cache.get(id).await {
            return Ok(Some(dp));
        }

        tracing::debug!("Cache miss for device state {:?}, fetching from Postgres", id);
        let Some(dp) = self.postgres.get_latest_for_device(id).await? else {
            return Ok(None);
        };
        self.current_cache.insert(*id, dp.clone()).await;
        Ok(Some(dp))
    }

    pub async fn get_all_data_points_in_range_ts_asc(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        self.postgres.get_all_data_points_in_range_ts_asc(range).await
    }
}
