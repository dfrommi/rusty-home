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
        DeviceAvailability, DeviceStateValue,
        adapter::{IncomingData, IncomingDataSource},
    },
};

use config::TadoChannel;

pub struct TadoIncomingDataSource {
    client: ClientWithMiddleware,
    zone_states_url: String,
    config: DeviceConfig<TadoChannel>,
    next_poll_at: tokio::time::Instant,
}

impl TadoIncomingDataSource {
    pub fn new(url: &str, home_id: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(None).new_tracing_client()?;
        let zone_states_url = format!("{}/api/v2/homes/{}/zoneStates", url, home_id);
        let config = DeviceConfig::new(&config::default_tado_state_config());

        Ok(Self {
            client,
            zone_states_url,
            config,
            next_poll_at: tokio::time::Instant::now(),
        })
    }
}

impl IncomingDataSource for TadoIncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let now = tokio::time::Instant::now();
            if now < self.next_poll_at {
                // Still within cooldown — sleep until next poll time.
                // May be cancelled by tokio::select!, but self.next_poll_at
                // survives, so progress is preserved on re-entry.
                tokio::time::sleep_until(self.next_poll_at).await;
                continue;
            }

            match self.poll_api().await {
                Ok(events) => {
                    // Set next poll time: 90s base + up to 30s jitter
                    let jitter = rand::random::<u64>() % 30;
                    self.next_poll_at = now + tokio::time::Duration::from_secs(90 + jitter);
                    if !events.is_empty() {
                        return Some(events);
                    }
                }
                Err(e) => {
                    tracing::error!("Tado API poll failed: {:?}", e);
                    // Retry after a short delay
                    self.next_poll_at = now + tokio::time::Duration::from_secs(10);
                }
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

            let mut zone_timestamps: Vec<DateTime> = Vec::new();

            for channel in channels {
                match channel {
                    TadoChannel::Temperature(variant) => {
                        if let Some(temp) = &zone_state.sensor_data_points.inside_temperature {
                            zone_timestamps.push(temp.timestamp);
                            events.push(IncomingData::StateValue(DataPoint::new(
                                DeviceStateValue::Temperature(*variant, DegreeCelsius(temp.celsius)),
                                temp.timestamp,
                            )));
                        }
                    }
                    TadoChannel::RelativeHumidity(variant) => {
                        if let Some(hum) = &zone_state.sensor_data_points.humidity {
                            zone_timestamps.push(hum.timestamp);
                            events.push(IncomingData::StateValue(DataPoint::new(
                                DeviceStateValue::RelativeHumidity(*variant, Percent(hum.percentage)),
                                hum.timestamp,
                            )));
                        }
                    }
                }
            }

            let last_seen = zone_timestamps.into_iter().max().unwrap_or_else(DateTime::now);

            events.push(IncomingData::ItemAvailability(DeviceAvailability {
                source: "Tado".to_string(),
                device_id: zone_id.clone(),
                last_seen,
                marked_offline: false,
            }));
        }

        Ok(events)
    }
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
