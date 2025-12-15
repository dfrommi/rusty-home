mod adapter;
mod domain;
mod service;

pub use domain::*;
use infrastructure::Mqtt;

use std::{collections::HashMap, sync::Arc};

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataPoint,
    },
    device_state::{
        adapter::{
            IncomingDataSource as _, db::DeviceStateRepository, tasmota::TasmotaIncomingDataSource,
            z2m::Z2mIncomingDataSource,
        },
        service::DeviceStateService,
    },
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
    tasmota_ds: TasmotaIncomingDataSource,
    z2m_ds: Z2mIncomingDataSource,
}

impl DeviceStateRunner {
    pub async fn new(pool: PgPool, mqtt_client: &mut Mqtt, tasmota_event_topic: &str, z2m_event_topic: &str) -> Self {
        let repo = DeviceStateRepository::new(pool);
        let tasmota_ds = TasmotaIncomingDataSource::new(mqtt_client, tasmota_event_topic).await;
        let z2m_ds = Z2mIncomingDataSource::new(mqtt_client, z2m_event_topic).await;

        let (event_tx, _event_rx) = broadcast::channel(100);

        let service = DeviceStateService::new(repo.clone(), event_tx.clone());

        DeviceStateRunner {
            service: Arc::new(service),
            tasmota_ds,
            z2m_ds,
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

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(updates) = self.tasmota_ds.recv_multi() => {
                    self.process_incoming_data(updates).await;
                }
                Some(updates) = self.z2m_ds.recv_multi() => {
                    self.process_incoming_data(updates).await;
                }
            }
        }
    }

    async fn process_incoming_data(&self, updates: Vec<adapter::IncomingData>) {
        for update in updates {
            match update {
                adapter::IncomingData::StateValue(data_point) => self.service.handle_state_update(data_point).await,
                adapter::IncomingData::ItemAvailability(device_availability) => {
                    self.service.handle_availability_update(device_availability).await
                }
            }
        }
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
