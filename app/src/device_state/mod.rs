mod adapter;
mod domain;
mod service;

pub use domain::*;
use infrastructure::{EventBus, EventListener, Mqtt};

use std::{collections::HashMap, sync::Arc};

use sqlx::PgPool;

use crate::{
    adapter::energy_meter::EnergyReading,
    command::CommandEvent,
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataPoint,
    },
    device_state::{
        adapter::{
            IncomingDataSource as _, db::DeviceStateRepository, energy_meter::EnergyMeterIncomingDataSource,
            homeassistant::HomeAssistantIncomingDataSource, internal::InternalDataSource,
            tasmota::TasmotaIncomingDataSource, z2m::Z2mIncomingDataSource,
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

pub struct DeviceStateModule {
    service: Arc<DeviceStateService>,
    event_bus: EventBus<DeviceStateEvent>,
    tasmota_ds: TasmotaIncomingDataSource,
    z2m_ds: Z2mIncomingDataSource,
    ha_ds: HomeAssistantIncomingDataSource,
    energy_meter_ds: EnergyMeterIncomingDataSource,
    internal_ds: InternalDataSource,
}

impl DeviceStateModule {
    pub async fn new(
        pool: PgPool,
        mqtt_client: &mut Mqtt,
        tasmota_event_topic: &str,
        z2m_event_topic: &str,
        ha_event_topic: &str,
        ha_url: &str,
        ha_token: &str,
        energy_reading_rx: EventListener<EnergyReading>,
        command_events: EventListener<CommandEvent>,
    ) -> Self {
        let repo = DeviceStateRepository::new(pool.clone());
        let tasmota_ds = TasmotaIncomingDataSource::new(mqtt_client, tasmota_event_topic).await;
        let z2m_ds = Z2mIncomingDataSource::new(mqtt_client, z2m_event_topic).await;
        let ha_ds = HomeAssistantIncomingDataSource::new(mqtt_client, ha_event_topic, ha_url, ha_token).await;
        let energy_meter_ds = EnergyMeterIncomingDataSource::new(pool, energy_reading_rx);
        let internal_ds = InternalDataSource::new(command_events);

        let event_bus = EventBus::new(128);

        let service = DeviceStateService::new(repo.clone(), event_bus.emitter());

        DeviceStateModule {
            service: Arc::new(service),
            event_bus,
            tasmota_ds,
            z2m_ds,
            ha_ds,
            energy_meter_ds,
            internal_ds,
        }
    }

    pub fn client(&self) -> DeviceStateClient {
        DeviceStateClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> EventListener<DeviceStateEvent> {
        self.event_bus.subscribe()
    }

    pub async fn run(mut self) {
        loop {
            //TODO expose error like "closed" when data-source gets refactored
            let updates = tokio::select! {
                updates = self.tasmota_ds.recv_multi() => updates,
                updates = self.z2m_ds.recv_multi() => updates,
                updates = self.ha_ds.recv_multi() => updates,
                updates = self.energy_meter_ds.recv_multi() => updates,
                updates = self.internal_ds.recv_multi() => updates,
            };

            if let Some(updates) = updates {
                self.process_incoming_data(updates).await;
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
