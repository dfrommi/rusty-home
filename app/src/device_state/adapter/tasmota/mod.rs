mod config;

use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{CurrentPowerUsage, DeviceAvailability, PowerAvailable, TotalEnergyConsumption};

use crate::core::DeviceConfig;

use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::core::unit::{KiloWattHours, Watt};
use crate::device_state::DeviceStateValue;
use crate::t;
use anyhow::bail;
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    EnergyMeter(CurrentPowerUsage, TotalEnergyConsumption),
    PowerToggle(PowerAvailable),
}

pub struct TasmotaIncomingDataSource {
    device_config: DeviceConfig<TasmotaChannel>,
    mqtt_receiver: MqttSubscription,
}

impl TasmotaIncomingDataSource {
    #[allow(clippy::expect_used)]
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> Self {
        let config = DeviceConfig::new(&config::default_tasmota_state_config());
        let rx = mqtt_client
            .subscribe_all(event_topic, &["tele/+/SENSOR", "stat/+/POWER"])
            .await
            .expect("Error subscribing to MQTT topic");

        Self {
            device_config: config,
            mqtt_receiver: rx,
        }
    }
}

impl IncomingDataSource<MqttInMessage, TasmotaChannel> for TasmotaIncomingDataSource {
    fn ds_name(&self) -> &str {
        "Tasmota"
    }

    async fn recv(&mut self) -> Option<MqttInMessage> {
        self.mqtt_receiver.recv().await
    }

    fn device_id(&self, msg: &MqttInMessage) -> Option<String> {
        if let Some(rest) = msg.topic.strip_prefix("tele/") {
            rest.strip_suffix("/SENSOR")
        } else if let Some(rest) = msg.topic.strip_prefix("stat/") {
            rest.strip_suffix("/POWER")
        } else {
            None
        }
        .map(|s| s.to_owned())
    }

    fn get_channels(&self, device_id: &str) -> &[TasmotaChannel] {
        self.device_config.get(device_id)
    }

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &TasmotaChannel,
        msg: &MqttInMessage,
    ) -> anyhow::Result<Vec<IncomingData>> {
        match channel {
            TasmotaChannel::EnergyMeter(power, energy) => {
                if !msg.topic.ends_with("/SENSOR") {
                    return Ok(vec![]);
                }

                let tele_message: TeleMessage = serde_json::from_str(&msg.payload)?;

                match &tele_message.payload {
                    TeleMessagePayload::EnergyReport(energy_report) => Ok(vec![
                        DataPoint::new(
                            DeviceStateValue::CurrentPowerUsage(*power, Watt(energy_report.power)),
                            tele_message.time,
                        )
                        .into(),
                        DataPoint::new(
                            DeviceStateValue::TotalEnergyConsumption(*energy, KiloWattHours(energy_report.total)),
                            tele_message.time,
                        )
                        .into(),
                        DeviceAvailability {
                            source: "Tasmota".to_string(),
                            device_id: device_id.to_string(),
                            last_seen: tele_message.time,
                            marked_offline: false,
                        }
                        .into(),
                    ]),

                    _ => Ok(vec![]),
                }
            }

            //No timestamp available in Tasmota. TODO: trigger update of state on startup
            TasmotaChannel::PowerToggle(powered) => {
                if !msg.topic.ends_with("/POWER") {
                    return Ok(vec![]);
                }

                match msg.payload.as_str() {
                    "ON" => Ok(vec![
                        DataPoint::new(DeviceStateValue::PowerAvailable(*powered, true), t!(now)).into(),
                    ]),
                    "OFF" => Ok(vec![
                        DataPoint::new(DeviceStateValue::PowerAvailable(*powered, false), t!(now)).into(),
                    ]),
                    _ => bail!("Unexpected payload for PowerToggle {}: {}", device_id, msg.payload),
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TeleMessage {
    #[serde(rename = "Time", deserialize_with = "datetime_format::deserialize")]
    time: DateTime,

    #[serde(flatten)]
    payload: TeleMessagePayload,
}

#[derive(Debug, Clone, serde::Deserialize)]
enum TeleMessagePayload {
    #[serde(rename = "ENERGY")]
    EnergyReport(EnergyReport),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EnergyReport {
    power: f64,
    total: f64,
}

mod datetime_format {
    use crate::core::time::DateTime;
    use chrono::{Local, NaiveDateTime, TimeZone, offset::LocalResult};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let naive = NaiveDateTime::parse_from_str(s, FORMAT).map_err(serde::de::Error::custom)?;
        let local = match Local.from_local_datetime(&naive) {
            LocalResult::Single(local) => local,
            LocalResult::Ambiguous(local, _) => local,
            LocalResult::None => return Err(serde::de::Error::custom("Invalid local datetime")),
        };

        Ok(DateTime::from(local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_energy_report() {
        let json = r#"{
            "Time":"2025-01-11T23:10:38",
            "ENERGY":{
                "TotalStartTime":"2022-11-28T13:42:21",
                "Total":6.096,
                "Yesterday":0.040,
                "Today":0.030,
                "Period":0,
                "Power":1,
                "ApparentPower":4,
                "ReactivePower":4,
                "Factor":0.25,
                "Voltage":230,
                "Current":0.019
            }
        }"#;

        let parsed: TeleMessage = serde_json::from_str(json).unwrap();
        let energy_report = match &parsed.payload {
            TeleMessagePayload::EnergyReport(report) => report,
            _ => panic!("Unexpected message type"),
        };

        assert_eq!(parsed.time.to_iso_string(), "2025-01-11T23:10:38+01:00");
        assert_eq!(energy_report.power, 1.0);
        assert_eq!(energy_report.total, 6.096);
    }
}
