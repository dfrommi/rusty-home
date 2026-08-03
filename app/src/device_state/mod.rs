mod adapter;
mod domain;
mod service;

pub use domain::*;
use infrastructure::{EventBus, EventListener, Mqtt};

use std::{collections::HashMap, sync::Arc};

use sqlx::PgPool;

use crate::{
    command::CommandEvent,
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{
        adapter::{
            IncomingDataSource, db::DeviceStateRepository, energy_meter::EnergyMeterIncomingDataSource,
            homeassistant::HomeAssistantIncomingDataSource, internal::InternalDataSource, tado::TadoIncomingDataSource,
            tasmota::TasmotaIncomingDataSource, z2m::Z2mIncomingDataSource,
        },
        service::DeviceStateService,
    },
    frontends::energy_meter::EnergyReading,
};

#[derive(Debug, Clone)]
pub enum DeviceStateEvent {
    Updated(DataPoint<DeviceStateValue>),
    Changed(#[allow(dead_code)] DataPoint<DeviceStateValue>),
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
pub struct DeviceAvailabilityStatus {
    pub source: String,
    pub item: String,
    pub last_seen_ago: Duration,
    pub is_offline: bool,
}

pub struct DeviceStateModule {
    service: Arc<DeviceStateService>,
    event_bus: EventBus<DeviceStateEvent>,
    tasmota_ds: TasmotaIncomingDataSource,
    z2m_ds: Z2mIncomingDataSource,
    ha_ds: HomeAssistantIncomingDataSource,
    energy_meter_ds: EnergyMeterIncomingDataSource,
    internal_ds: InternalDataSource,
    tado_ds: TadoIncomingDataSource,
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
        tado_url: &str,
        tado_home_id: &str,
    ) -> Self {
        let repo = DeviceStateRepository::new(pool.clone());
        let tasmota_ds = TasmotaIncomingDataSource::new(mqtt_client, tasmota_event_topic).await;
        let z2m_ds = Z2mIncomingDataSource::new(mqtt_client, z2m_event_topic).await;
        let ha_ds = HomeAssistantIncomingDataSource::new(mqtt_client, ha_event_topic, ha_url, ha_token).await;
        let energy_meter_ds = EnergyMeterIncomingDataSource::new(pool, energy_reading_rx);
        let internal_ds = InternalDataSource::new(command_events);
        let tado_ds = TadoIncomingDataSource::new(tado_url, tado_home_id).expect("Error creating Tado adapter");

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
            tado_ds,
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
            let (source, updates) = tokio::select! {
                updates = self.tasmota_ds.recv_multi() => ("Tasmota", updates),
                updates = self.z2m_ds.recv_multi() => ("Z2M", updates),
                updates = self.ha_ds.recv_multi() => ("HomeAssistant", updates),
                updates = self.energy_meter_ds.recv_multi() => ("EnergyMeter", updates),
                updates = self.internal_ds.recv_multi() => ("Internal", updates),
                updates = self.tado_ds.recv_multi() => ("Tado", updates),
            };

            let Some(updates) = updates else {
                tracing::error!("Device state data source {} closed; stopping device state module", source);
                return;
            };

            self.process_incoming_data(updates).await;
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
    pub async fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        self.service.get_current_for_all().await
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<HashMap<DeviceStateId, DataFrame<DeviceStateValue>>> {
        Ok(group_by_device_id(self.service.get_all_data_points_in_range(range).await?))
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        self.service.get_item_availabilities().await
    }
}

fn group_by_device_id(
    data_points: Vec<DataPoint<DeviceStateValue>>,
) -> HashMap<DeviceStateId, DataFrame<DeviceStateValue>> {
    data_points.into_iter().fold(HashMap::new(), |mut acc, dp| {
        let id = DeviceStateId::from(&dp.value);
        acc.entry(id).or_insert_with(DataFrame::empty).insert(dp);
        acc
    })
}

#[cfg(test)]
mod tests {
    use crate::{core::unit::DegreeCelsius, t};

    use super::*;

    #[test]
    fn test_group_by_device_id() {
        let dps = vec![
            DataPoint::new(
                DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.0)),
                t!(30 minutes ago),
            ),
            DataPoint::new(
                DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.0)),
                t!(20 minutes ago),
            ),
            DataPoint::new(
                DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.5)),
                t!(20 minutes ago),
            ),
            DataPoint::new(
                DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.5)),
                t!(25 minutes ago),
            ),
            DataPoint::new(
                DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0)),
                t!(10 minutes ago),
            ),
        ];

        let grouped = group_by_device_id(dps);

        assert_eq!(grouped.len(), 2);
        let living_room_df = grouped
            .get(&DeviceStateId::Temperature(Temperature::LivingRoom))
            .expect("Living room DataFrame not found");
        let bedroom_df = grouped
            .get(&DeviceStateId::Temperature(Temperature::Bedroom))
            .expect("Bedroom DataFrame not found");

        assert_eq!(living_room_df.len(), 3);
        assert_eq!(
            living_room_df.prev_or_at(t!(15 minutes ago)).unwrap().value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.5))
        );

        assert_eq!(bedroom_df.len(), 2);

        println!("Grouped DataFrames: {:?}", grouped);
    }
}
