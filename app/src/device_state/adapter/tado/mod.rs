mod config;

use std::collections::HashMap;

use anyhow::Context;
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

use crate::{
    core::{
        DeviceConfig,
        time::DateTime,
        timeseries::DataPoint,
        unit::{DegreeCelsius, Percent},
    },
    device_state::{
        DeviceAvailability, DeviceAvailabilityItem, DeviceStateValue,
        adapter::{IncomingData, IncomingDataSource},
    },
};

use config::TadoChannel;

const AVAILABILITY_SOURCE: &str = "Tado";

pub struct TadoIncomingDataSource {
    client: ClientWithMiddleware,
    zone_states_url: String,
    zones_url: String,
    config: DeviceConfig<TadoChannel>,
    next_zone_states_poll: tokio::time::Instant,
    next_zones_poll: tokio::time::Instant,
}

impl TadoIncomingDataSource {
    pub fn new(url: &str, home_id: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(None).new_tracing_client()?;
        let zone_states_url = format!("{}/api/v2/homes/{}/zoneStates", url, home_id);
        let zones_url = format!("{}/api/v2/homes/{}/zones", url, home_id);
        let config = DeviceConfig::new(&config::default_tado_state_config());

        Ok(Self {
            client,
            zone_states_url,
            zones_url,
            config,
            next_zone_states_poll: tokio::time::Instant::now(),
            next_zones_poll: tokio::time::Instant::now(),
        })
    }
}

impl IncomingDataSource for TadoIncomingDataSource {
    fn availability_items(&self) -> Vec<DeviceAvailabilityItem> {
        self.config
            .keys()
            .map(|item| DeviceAvailabilityItem::new(AVAILABILITY_SOURCE, item))
            .collect()
    }

    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let now = tokio::time::Instant::now();
            let next = self.next_zone_states_poll.min(self.next_zones_poll);
            if now < next {
                tokio::time::sleep_until(next).await;
                continue;
            }

            let now = tokio::time::Instant::now();
            let mut events = Vec::new();

            if now >= self.next_zones_poll {
                match self.poll_zones().await {
                    Ok(mut zone_events) => {
                        let jitter = rand::random::<u64>() % 60;
                        self.next_zones_poll += tokio::time::Duration::from_secs(3600 + jitter);
                        events.append(&mut zone_events);
                    }
                    Err(e) => {
                        tracing::error!("Tado zones poll failed: {:?}", e);
                        self.next_zones_poll = now + tokio::time::Duration::from_secs(10);
                    }
                }
            }

            if now >= self.next_zone_states_poll {
                match self.poll_api().await {
                    Ok(mut state_events) => {
                        let jitter = rand::random::<u64>() % 30;
                        self.next_zone_states_poll += tokio::time::Duration::from_secs(90 + jitter);
                        events.append(&mut state_events);
                    }
                    Err(e) => {
                        tracing::error!("Tado zoneStates poll failed: {:?}", e);
                        self.next_zone_states_poll = now + tokio::time::Duration::from_secs(10);
                    }
                }
            }

            if !events.is_empty() {
                return Some(events);
            }
        }
    }
}

impl TadoIncomingDataSource {
    async fn poll_api(&self) -> anyhow::Result<Vec<IncomingData>> {
        let response = self
            .client
            .get(&self.zone_states_url)
            .send()
            .await
            .context("Tado zoneStates request failed")?;

        let body: ZoneStatesResponse = response
            .json()
            .await
            .context("Tado zoneStates deserialization failed")?;

        let mut events = Vec::new();

        for (zone_id, zone_state) in &body.zone_states {
            let Some(channels) = self.config.get_optional(zone_id) else {
                continue;
            };

            for channel in channels {
                match channel {
                    TadoChannel::Temperature(variant) => {
                        if let Some(temp) = &zone_state.sensor_data_points.inside_temperature {
                            events.push(IncomingData::StateValue(DataPoint::new(
                                DeviceStateValue::Temperature(*variant, DegreeCelsius(temp.celsius)),
                                temp.timestamp,
                            )));
                        }
                    }
                    TadoChannel::RelativeHumidity(variant) => {
                        if let Some(hum) = &zone_state.sensor_data_points.humidity {
                            events.push(IncomingData::StateValue(DataPoint::new(
                                DeviceStateValue::RelativeHumidity(*variant, Percent(hum.percentage)),
                                hum.timestamp,
                            )));
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    async fn poll_zones(&self) -> anyhow::Result<Vec<IncomingData>> {
        let response = self
            .client
            .get(&self.zones_url)
            .send()
            .await
            .context("Tado zones request failed")?;

        let zones: Vec<ZoneInfo> = response.json().await.context("Tado zones deserialization failed")?;

        let mut events = Vec::new();

        for zone in &zones {
            let zone_id = zone.id.to_string();
            if self.config.get_optional(&zone_id).is_none() {
                continue;
            }

            let Some(ru01) = zone.devices.iter().find(|d| d.device_type == "RU01") else {
                continue;
            };

            events.push(IncomingData::ItemAvailability(DeviceAvailability {
                item: DeviceAvailabilityItem::new(AVAILABILITY_SOURCE, zone_id),
                last_seen: ru01.connection_state.timestamp,
                marked_offline: !ru01.connection_state.value,
            }));
        }

        Ok(events)
    }
}

#[derive(Deserialize)]
struct ZoneInfo {
    id: i64,
    devices: Vec<DeviceInfo>,
}

#[derive(Deserialize)]
struct DeviceInfo {
    #[serde(rename = "deviceType")]
    device_type: String,
    #[serde(rename = "connectionState")]
    connection_state: ConnectionState,
}

#[derive(Deserialize)]
struct ConnectionState {
    value: bool,
    timestamp: DateTime,
}

#[derive(Deserialize)]
struct ZoneStatesResponse {
    #[serde(rename = "zoneStates")]
    zone_states: HashMap<String, ZoneState>,
}

#[derive(Deserialize)]
struct ZoneState {
    #[serde(rename = "sensorDataPoints")]
    sensor_data_points: SensorDataPoints,
}

#[derive(Deserialize)]
struct SensorDataPoints {
    #[serde(rename = "insideTemperature")]
    inside_temperature: Option<TemperaturePoint>,
    humidity: Option<HumidityPoint>,
}

#[derive(Deserialize)]
struct TemperaturePoint {
    celsius: f64,
    timestamp: DateTime,
}

#[derive(Deserialize)]
struct HumidityPoint {
    percentage: f64,
    timestamp: DateTime,
}
