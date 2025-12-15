mod adapter;
mod domain;
mod service;

pub use domain::*;

use std::{collections::HashMap, sync::Arc};

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataPoint,
    },
    device_state::{adapter::db::DeviceStateRepository, service::DeviceStateService},
};

#[derive(Debug, Clone)]
pub enum DeviceStateEvent {
    Updated(DataPoint<DeviceStateValue>),
    Changed(DataPoint<DeviceStateValue>),
}

//Trait would be better, but no dyn support for async fn makes it too cumbersome
#[derive(Clone)]
pub struct DeviceStateClient {
    service: Arc<DeviceStateService>,
}

#[derive(Debug, Clone)]
pub struct DeviceAvailability {
    pub source: String,
    pub device_id: String,
    pub last_seen: DateTime,
    pub marked_offline: bool,
}

#[derive(Debug, Clone)]
pub struct OfflineItem {
    pub source: String,
    pub item: String,
    pub duration: Duration,
}

pub struct DeviceStateRunner {
    service: Arc<DeviceStateService>,
}

impl DeviceStateRunner {
    pub fn new(pool: PgPool) -> Self {
        let repo = DeviceStateRepository::new(pool);

        let (event_tx, _event_rx) = broadcast::channel(100);

        let service = DeviceStateService::new(repo.clone(), event_tx.clone());

        DeviceStateRunner {
            service: Arc::new(service),
        }
    }

    pub fn client(&self) -> DeviceStateClient {
        DeviceStateClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DeviceStateEvent> {
        self.service.subscribe()
    }
}

impl DeviceStateClient {
    pub async fn update_state(&self, data_point: DataPoint<DeviceStateValue>) -> anyhow::Result<()> {
        self.service.handle_state_update(data_point).await;
        Ok(())
    }

    pub async fn update_availability(&self, availability: DeviceAvailability) -> anyhow::Result<()> {
        self.service.handle_availability_update(availability).await;
        Ok(())
    }

    pub async fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        self.service.get_current_for_all().await
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        self.service.get_all_data_points_in_range(range).await
    }

    pub async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        self.service.get_offline_items().await
    }
}
