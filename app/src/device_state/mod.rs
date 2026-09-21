mod adapter;
mod domain;
mod service;

use anyhow::Context;
pub use domain::*;
use infrastructure::{EventBus, EventListener, Mqtt};
use serde::Deserialize;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use sqlx::PgPool;

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{
        adapter::{
            IncomingDataSource, db::DeviceStateRepository, energy_meter::EnergyMeterIncomingDataSource,
            homeassistant::HomeAssistantIncomingDataSource, lgtv::LgtvIncomingDataSource, tado::TadoIncomingDataSource,
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

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceAvailabilityConfig {
    pub default_offline_after: Duration,
    #[serde(default)]
    pub overrides: Vec<DeviceAvailabilityOverride>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceAvailabilityOverride {
    pub source: String,
    pub item: String,
    pub offline_after: Duration,
}

impl DeviceAvailabilityConfig {
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();

        for override_config in &self.overrides {
            let key = (&override_config.source, &override_config.item);
            if !seen.insert(key) {
                return Err(format!(
                    "Duplicate device availability override for {}/{}",
                    override_config.source, override_config.item
                ));
            }
        }

        Ok(())
    }

    pub fn warn_unknown_items(&self, items: &HashSet<DeviceAvailabilityItem>) {
        for override_config in &self.overrides {
            let item = DeviceAvailabilityItem::new(&override_config.source, &override_config.item);
            if !items.contains(&item) {
                tracing::warn!(
                    source = %override_config.source,
                    item = %override_config.item,
                    "Ignoring device availability override for unknown item"
                );
            }
        }
    }

    pub fn offline_after(&self, source: &str, item: &str) -> &Duration {
        self.overrides
            .iter()
            .find(|override_config| override_config.source == source && override_config.item == item)
            .map(|override_config| &override_config.offline_after)
            .unwrap_or(&self.default_offline_after)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceAvailabilityItem {
    pub source: String,
    pub item: String,
}

impl DeviceAvailabilityItem {
    pub fn new(source: impl Into<String>, item: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            item: item.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceAvailability {
    pub item: DeviceAvailabilityItem,
    pub last_seen: DateTime,
    pub marked_offline: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceAvailabilityStatus {
    pub source: String,
    pub item: String,
    pub last_seen_ago: Duration,
    pub is_offline: bool,
    pub disabled: bool,
}

pub struct DeviceStateModule {
    service: Arc<DeviceStateService>,
    event_bus: EventBus<DeviceStateEvent>,
    tasmota_ds: TasmotaIncomingDataSource,
    z2m_ds: Z2mIncomingDataSource,
    ha_ds: HomeAssistantIncomingDataSource,
    lgtv_ds: LgtvIncomingDataSource,
    energy_meter_ds: EnergyMeterIncomingDataSource,
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
        lgtv_base_topic: &str,
        energy_reading_rx: EventListener<EnergyReading>,
        tado_url: &str,
        tado_home_id: &str,
        availability_config: DeviceAvailabilityConfig,
    ) -> anyhow::Result<Self> {
        let repo = DeviceStateRepository::new(pool.clone(), availability_config);
        let tasmota_ds = TasmotaIncomingDataSource::new(mqtt_client, tasmota_event_topic).await?;
        let z2m_ds = Z2mIncomingDataSource::new(mqtt_client, z2m_event_topic).await?;
        let ha_ds = HomeAssistantIncomingDataSource::new(mqtt_client, ha_event_topic, ha_url, ha_token).await?;
        let lgtv_ds = LgtvIncomingDataSource::new(mqtt_client, lgtv_base_topic).await?;
        let energy_meter_ds = EnergyMeterIncomingDataSource::new(pool, energy_reading_rx);
        let tado_ds = TadoIncomingDataSource::new(tado_url, tado_home_id).context("Error creating Tado adapter")?;

        let event_bus = EventBus::new(128);

        let service = DeviceStateService::new(repo.clone(), event_bus.emitter());

        Ok(DeviceStateModule {
            service: Arc::new(service),
            event_bus,
            tasmota_ds,
            z2m_ds,
            ha_ds,
            lgtv_ds,
            energy_meter_ds,
            tado_ds,
        })
    }

    pub fn client(&self) -> DeviceStateClient {
        DeviceStateClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> EventListener<DeviceStateEvent> {
        self.event_bus.subscribe()
    }

    pub async fn initialize_availability(&self) -> anyhow::Result<()> {
        let mut items = HashSet::new();
        items.extend(self.tasmota_ds.availability_items());
        items.extend(self.z2m_ds.availability_items());
        items.extend(self.ha_ds.availability_items());
        items.extend(self.lgtv_ds.availability_items());
        items.extend(self.energy_meter_ds.availability_items());
        items.extend(self.tado_ds.availability_items());

        self.service.initialize_availability(items).await
    }

    pub async fn run(mut self) {
        loop {
            let (source, updates) = tokio::select! {
                updates = self.tasmota_ds.recv_multi() => ("Tasmota", updates),
                updates = self.z2m_ds.recv_multi() => ("Z2M", updates),
                updates = self.ha_ds.recv_multi() => ("HomeAssistant", updates),
                updates = self.lgtv_ds.recv_multi() => ("LGTV", updates),
                updates = self.energy_meter_ds.recv_multi() => ("EnergyMeter", updates),
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::{core::unit::DegreeCelsius, t};

    use super::*;

    #[test]
    fn availability_config_uses_override_or_default_duration() {
        let config = DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![DeviceAvailabilityOverride {
                source: "HA".to_string(),
                item: "sensor.home_temperature".to_string(),
                offline_after: t!(3 hours),
            }],
        };

        assert_eq!(config.offline_after("HA", "sensor.home_temperature"), &t!(3 hours));
        assert_eq!(config.offline_after("Tado", "1"), &t!(1 hours));
    }

    #[test]
    fn availability_config_deserializes_toml_format() {
        let settings = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
                    [device_availability]
                    default_offline_after = "PT1H"

                    [[device_availability.overrides]]
                    source = "HA"
                    item = "sensor.home_temperature"
                    offline_after = "PT3H"
                "#,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap();

        let availability_config: DeviceAvailabilityConfig = settings.get("device_availability").unwrap();

        assert_eq!(availability_config.offline_after("HA", "sensor.home_temperature"), &t!(3 hours));
    }

    #[test]
    fn availability_config_rejects_duplicate_overrides() {
        let config = DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![
                DeviceAvailabilityOverride {
                    source: "HA".to_string(),
                    item: "sensor.home_temperature".to_string(),
                    offline_after: t!(3 hours),
                },
                DeviceAvailabilityOverride {
                    source: "HA".to_string(),
                    item: "sensor.home_temperature".to_string(),
                    offline_after: t!(2 hours),
                },
            ],
        };

        assert!(config.validate().is_err());
    }

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
