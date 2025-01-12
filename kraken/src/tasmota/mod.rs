use api::state::{ChannelValue, CurrentPowerUsage, TotalEnergyConsumption};
use support::{
    mqtt::MqttInMessage,
    time::DateTime,
    unit::{KiloWattHours, Watt},
    DataPoint,
};

use crate::core::{IncomingData, IncomingMqttEventParser, ItemAvailability};

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
}

pub struct TasmotaMqttParser {
    tele_base_topic: String,
}

impl TasmotaMqttParser {
    pub fn new(base_topic: String) -> Self {
        let tele_base_topic = format!("{}/tele", base_topic.trim_matches('/'));
        Self { tele_base_topic }
    }
}

impl IncomingMqttEventParser<TasmotaChannel> for TasmotaMqttParser {
    fn topic_pattern(&self) -> String {
        format!("{}/+/SENSOR", &self.tele_base_topic)
    }

    fn device_id(&self, msg: &MqttInMessage) -> String {
        let topic = &msg.topic;

        topic
            .strip_prefix(&self.tele_base_topic)
            .unwrap_or(topic)
            .strip_suffix("/SENSOR")
            .unwrap_or(topic)
            .trim_matches('/')
            .to_owned()
    }

    fn get_events(
        &self,
        device_id: &str,
        channel: &TasmotaChannel,
        payload: &str,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let tele_message: TeleMessage = serde_json::from_str(payload)?;

        match (channel, &tele_message.payload) {
            (
                TasmotaChannel::PowerPlug(power, energy),
                TeleMessagePayload::EnergyReport(energy_report),
            ) => Ok(vec![
                DataPoint::new(
                    ChannelValue::CurrentPowerUsage(power.clone(), Watt(energy_report.power)),
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
    use chrono::{offset::LocalResult, Local, NaiveDateTime, TimeZone};
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
