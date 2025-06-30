use anyhow::bail;
use api::state::ChannelValue;
use infrastructure::MqttInMessage;
use support::{
    DataPoint, t,
    time::DateTime,
    unit::{KiloWattHours, Watt},
};
use tokio::sync::mpsc;

use crate::core::{DeviceConfig, IncomingData, IncomingDataSource, ItemAvailability};

use super::TasmotaChannel;

pub struct TasmotaIncomingDataSource {
    tele_base_topic: String,
    stat_base_topic: String,
    device_config: DeviceConfig<TasmotaChannel>,
    mqtt_receiver: mpsc::Receiver<MqttInMessage>,
}

impl TasmotaIncomingDataSource {
    pub fn new(
        tele_base_topic: String,
        stat_base_topic: String,
        config: DeviceConfig<TasmotaChannel>,
        mqtt_rx: mpsc::Receiver<MqttInMessage>,
    ) -> Self {
        let tele_base_topic = tele_base_topic.trim_matches('/').to_string();
        let stat_base_topic = stat_base_topic.trim_matches('/').to_string();

        Self {
            tele_base_topic,
            stat_base_topic,
            device_config: config,
            mqtt_receiver: mqtt_rx,
        }
    }
}

impl IncomingDataSource<MqttInMessage, TasmotaChannel> for TasmotaIncomingDataSource {
    async fn recv(&mut self) -> Option<MqttInMessage> {
        self.mqtt_receiver.recv().await
    }

    fn device_id(&self, msg: &MqttInMessage) -> Option<String> {
        let topic = &msg.topic;

        if topic.starts_with(self.tele_base_topic.as_str()) {
            topic
                .strip_prefix(&self.tele_base_topic)
                .and_then(|topic| topic.strip_suffix("/SENSOR"))
        } else if topic.starts_with(self.stat_base_topic.as_str()) {
            topic
                .strip_prefix(&self.stat_base_topic)
                .and_then(|topic| topic.strip_suffix("/POWER"))
        } else {
            None
        }
        .map(|topic| topic.trim_matches('/').to_owned())
    }

    fn get_channels(&self, device_id: &str) -> &[TasmotaChannel] {
        self.device_config.get(device_id)
    }

    fn to_incoming_data(
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
                            ChannelValue::CurrentPowerUsage(
                                power.clone(),
                                Watt(energy_report.power),
                            ),
                            tele_message.time,
                        )
                        .into(),
                        DataPoint::new(
                            ChannelValue::TotalEnergyConsumption(
                                energy.clone(),
                                KiloWattHours(energy_report.total),
                            ),
                            tele_message.time,
                        )
                        .into(),
                        ItemAvailability {
                            source: "Tasmota".to_string(),
                            item: device_id.to_string(),
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
                        DataPoint::new(ChannelValue::Powered(powered.clone(), true), t!(now))
                            .into(),
                    ]),
                    "OFF" => Ok(vec![
                        DataPoint::new(ChannelValue::Powered(powered.clone(), false), t!(now))
                            .into(),
                    ]),
                    _ => bail!(
                        "Unexpected payload for PowerToggle {}: {}",
                        device_id,
                        msg.payload
                    ),
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
    use chrono::{Local, NaiveDateTime, TimeZone, offset::LocalResult};
    use serde::{self, Deserialize, Deserializer};
    use support::time::DateTime;

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
