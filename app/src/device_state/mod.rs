mod adapter;
mod availability_service;
mod domain;
mod state_service;

use anyhow::Context;
pub use domain::*;
use infrastructure::{EventBus, EventListener, Mqtt, SQLiteDatabase};
use serde::Deserialize;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    core::{
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{
        adapter::{
            IncomingDataSource,
            db::{SQLiteDeviceAvailabilityRepository, SQLiteDeviceStateRepository},
            energy_meter::EnergyMeterIncomingDataSource,
            homeassistant::HomeAssistantIncomingDataSource,
            lgtv::LgtvIncomingDataSource,
            tado::TadoIncomingDataSource,
            tasmota::TasmotaIncomingDataSource,
            z2m::Z2mIncomingDataSource,
        },
        availability_service::DeviceAvailabilityService,
        state_service::DeviceStateService,
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
    availability_service: Arc<DeviceAvailabilityService>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAvailabilityRecord {
    pub item: DeviceAvailabilityItem,
    pub last_seen: DateTime,
    pub marked_offline: bool,
    pub entry_updated: DateTime,
    pub disabled: bool,
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
    availability_service: Arc<DeviceAvailabilityService>,
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
        mqtt_client: &mut Mqtt,
        tasmota_event_topic: &str,
        z2m_event_topic: &str,
        ha_event_topic: &str,
        ha_url: &str,
        ha_token: &str,
        lgtv_base_topic: &str,
        energy_reading_rx: EventListener<DataPoint<EnergyReading>>,
        tado_url: &str,
        tado_home_id: &str,
        availability_config: DeviceAvailabilityConfig,
        sqlite_database: SQLiteDatabase,
    ) -> anyhow::Result<Self> {
        let tasmota_ds = TasmotaIncomingDataSource::new(mqtt_client, tasmota_event_topic).await?;
        let z2m_ds = Z2mIncomingDataSource::new(mqtt_client, z2m_event_topic).await?;
        let ha_ds = HomeAssistantIncomingDataSource::new(mqtt_client, ha_event_topic, ha_url, ha_token).await?;
        let lgtv_ds = LgtvIncomingDataSource::new(mqtt_client, lgtv_base_topic).await?;
        let energy_meter_ds = EnergyMeterIncomingDataSource::new(energy_reading_rx);
        let tado_ds = TadoIncomingDataSource::new(tado_url, tado_home_id).context("Error creating Tado adapter")?;

        let event_bus = EventBus::new(128);

        let (sqlite_availability_repo, sqlite_state_repo) = {
            let connection = sqlite_database.for_writing()?;
            (
                SQLiteDeviceAvailabilityRepository::new(&connection)?,
                SQLiteDeviceStateRepository::new(&connection)?,
            )
        };
        let availability_service =
            DeviceAvailabilityService::new(sqlite_availability_repo, sqlite_database.clone(), availability_config);
        let service = DeviceStateService::new(sqlite_state_repo, sqlite_database, event_bus.emitter());
        Ok(DeviceStateModule {
            service: Arc::new(service),
            availability_service: Arc::new(availability_service),
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
            availability_service: self.availability_service.clone(),
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

        self.availability_service.initialize(items).await
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
                adapter::IncomingData::StateValue(data_point) => {
                    if let Err(error) = self.service.handle_state_update(data_point).await {
                        tracing::error!("Error saving device state update: {error:#}");
                    }
                }
                adapter::IncomingData::ItemAvailability(device_availability) => {
                    if let Err(error) = self.availability_service.handle_update(device_availability).await {
                        tracing::error!("Error saving device availability update: {error:#}");
                    }
                }
            }
        }
    }
}

impl DeviceStateClient {
    pub fn get_latest_for_device(&self, id: &DeviceStateId) -> anyhow::Result<Option<DataPoint<DeviceStateValue>>> {
        self.service.get_latest_for_device(id)
    }

    pub fn get_state_since(
        &self,
        id: &DeviceStateId,
        since: DateTime,
    ) -> anyhow::Result<Option<DataFrame<DeviceStateValue>>> {
        self.service.get_state_since(id, since)
    }

    #[tracing::instrument(skip(self))]
    pub fn get_current_for_all(&self) -> anyhow::Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        self.service.get_current_for_all()
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        self.availability_service.get_item_availabilities().await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::t;

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
}
