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

impl IncomingDataSource for TasmotaIncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let msg = self.mqtt_receiver.recv().await?;

            let Some(topic) = parse_tasmota_topic(&msg.topic) else {
                continue;
            };

            let device_id = topic.device_id();
            let Some(channels) = self.device_config.get_optional(device_id) else {
                continue;
            };

            if channels.is_empty() {
                continue;
            }

            tracing::debug!("Received Tasmota event for devices {}: {:?}", device_id, channels);

            return Some(parse_configured_channels(&topic, channels, &msg));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TasmotaTopic<'a> {
    Sensor { device_id: &'a str },
    Power { device_id: &'a str },
}

impl TasmotaTopic<'_> {
    fn device_id(&self) -> &str {
        match self {
            Self::Sensor { device_id } | Self::Power { device_id } => device_id,
        }
    }
}

fn parse_tasmota_topic(topic: &str) -> Option<TasmotaTopic<'_>> {
    if let Some(rest) = topic.strip_prefix("tele/") {
        rest.strip_suffix("/SENSOR")
            .map(|device_id| TasmotaTopic::Sensor { device_id })
    } else if let Some(rest) = topic.strip_prefix("stat/") {
        rest.strip_suffix("/POWER")
            .map(|device_id| TasmotaTopic::Power { device_id })
    } else {
        None
    }
}

fn parse_configured_channels(
    topic: &TasmotaTopic<'_>,
    channels: &[TasmotaChannel],
    msg: &MqttInMessage,
) -> Vec<IncomingData> {
    let mut incoming_data = vec![];
    let device_id = topic.device_id();

    for channel in channels {
        match parse_tasmota_channel(device_id, channel, topic, &msg.payload) {
            Ok(events) => incoming_data.extend(events),
            Err(e) => {
                tracing::error!(
                    "Error parsing Tasmota event for channel {:?} with payload {:?}: {:?}",
                    channel,
                    msg,
                    e
                );
            }
        }
    }

    incoming_data
}

fn parse_tasmota_channel(
    device_id: &str,
    channel: &TasmotaChannel,
    topic: &TasmotaTopic<'_>,
    payload: &str,
) -> anyhow::Result<Vec<IncomingData>> {
    match (topic, channel) {
        (TasmotaTopic::Sensor { .. }, TasmotaChannel::EnergyMeter(power, energy)) => {
            parse_energy_meter(device_id, *power, *energy, payload)
        }
        (TasmotaTopic::Power { .. }, TasmotaChannel::PowerToggle(powered)) => {
            parse_power_toggle(device_id, *powered, payload)
        }
        (TasmotaTopic::Power { .. }, TasmotaChannel::EnergyMeter(_, _))
        | (TasmotaTopic::Sensor { .. }, TasmotaChannel::PowerToggle(_)) => Ok(vec![]),
    }
}

fn parse_energy_meter(
    device_id: &str,
    power: CurrentPowerUsage,
    energy: TotalEnergyConsumption,
    payload: &str,
) -> anyhow::Result<Vec<IncomingData>> {
    let tele_message: TeleMessage = serde_json::from_str(payload)?;

    let Some(energy_report) = &tele_message.energy_report else {
        return Ok(vec![]);
    };

    Ok(vec![
        DataPoint::new(
            DeviceStateValue::CurrentPowerUsage(power, Watt(energy_report.power)),
            tele_message.time,
        )
        .into(),
        DataPoint::new(
            DeviceStateValue::TotalEnergyConsumption(energy, KiloWattHours(energy_report.total)),
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
    ])
}

//No timestamp available in Tasmota. TODO: trigger update of state on startup
fn parse_power_toggle(device_id: &str, powered: PowerAvailable, payload: &str) -> anyhow::Result<Vec<IncomingData>> {
    match payload {
        "ON" => Ok(vec![
            DataPoint::new(DeviceStateValue::PowerAvailable(powered, true), t!(now)).into(),
        ]),
        "OFF" => Ok(vec![
            DataPoint::new(DeviceStateValue::PowerAvailable(powered, false), t!(now)).into(),
        ]),
        _ => bail!("Unexpected payload for PowerToggle {}: {}", device_id, payload),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TeleMessage {
    #[serde(rename = "Time", deserialize_with = "datetime_format::deserialize")]
    time: DateTime,

    #[serde(rename = "ENERGY")]
    energy_report: Option<EnergyReport>,
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

    fn state_values(items: Vec<IncomingData>) -> Vec<DeviceStateValue> {
        items
            .into_iter()
            .filter_map(|item| match item {
                IncomingData::StateValue(data_point) => Some(data_point.value),
                IncomingData::ItemAvailability(_) => None,
            })
            .collect()
    }

    fn energy_payload() -> &'static str {
        r#"{
            "Time":"2025-01-11T23:10:38",
            "ENERGY":{
                "Total":6.096,
                "Power":1
            }
        }"#
    }

    #[test]
    fn supported_tasmota_topics_are_explicit() {
        assert_eq!(
            parse_tasmota_topic("tele/apple-tv/SENSOR"),
            Some(TasmotaTopic::Sensor { device_id: "apple-tv" })
        );
        assert_eq!(
            parse_tasmota_topic("stat/irheater/POWER"),
            Some(TasmotaTopic::Power { device_id: "irheater" })
        );
        assert_eq!(parse_tasmota_topic("tele/apple-tv/POWER"), None);
    }

    #[test]
    fn parses_configured_power_channel_after_noop_channel() {
        let msg = MqttInMessage {
            topic: "stat/irheater/POWER".to_string(),
            payload: "ON".to_string(),
        };
        let topic = parse_tasmota_topic(&msg.topic).unwrap();
        let channels = vec![
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::InfraredHeater, TotalEnergyConsumption::InfraredHeater),
            TasmotaChannel::PowerToggle(PowerAvailable::InfraredHeater),
        ];

        let values = state_values(parse_configured_channels(&topic, &channels, &msg));

        assert_eq!(
            values,
            vec![DeviceStateValue::PowerAvailable(PowerAvailable::InfraredHeater, true)]
        );
    }

    #[test]
    fn ignores_irrelevant_topic_channel_combinations() {
        let sensor_topic = TasmotaTopic::Sensor { device_id: "irheater" };
        let power_topic = TasmotaTopic::Power { device_id: "irheater" };

        assert!(
            parse_tasmota_channel(
                "irheater",
                &TasmotaChannel::PowerToggle(PowerAvailable::InfraredHeater),
                &sensor_topic,
                energy_payload(),
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            parse_tasmota_channel(
                "irheater",
                &TasmotaChannel::EnergyMeter(CurrentPowerUsage::InfraredHeater, TotalEnergyConsumption::InfraredHeater),
                &power_topic,
                "ON",
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn non_energy_telemetry_is_noop() {
        let payload = r#"{
            "Time":"2025-01-11T23:10:38",
            "StatusSNS":{}
        }"#;

        let items = parse_energy_meter(
            "irheater",
            CurrentPowerUsage::InfraredHeater,
            TotalEnergyConsumption::InfraredHeater,
            payload,
        )
        .unwrap();

        assert!(items.is_empty());
    }

    #[test]
    fn unknown_power_payload_is_parse_error() {
        assert!(parse_power_toggle("irheater", PowerAvailable::InfraredHeater, "UNKNOWN").is_err());
    }

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
        let energy_report = parsed.energy_report.unwrap();

        assert_eq!(parsed.time.to_iso_string(), "2025-01-11T23:10:38+01:00");
        assert_eq!(energy_report.power, 1.0);
        assert_eq!(energy_report.total, 6.096);
    }
}
