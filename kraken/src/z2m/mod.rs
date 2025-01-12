use api::state::{
    ChannelValue, CurrentPowerUsage, Opened, RelativeHumidity, Temperature, TotalEnergyConsumption,
};
use support::{
    mqtt::MqttInMessage,
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent, Watt},
    DataPoint,
};

use crate::core::{IncomingData, IncomingMqttEventParser, ItemAvailability};

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
}

pub struct Z2mMqttParser {
    base_topic: String,
}

impl Z2mMqttParser {
    pub fn new(base_topic: String) -> Self {
        Self {
            base_topic: base_topic.trim_matches('/').to_owned(),
        }
    }
}

impl IncomingMqttEventParser<Z2mChannel> for Z2mMqttParser {
    fn topic_pattern(&self) -> String {
        format!("{}/#", &self.base_topic)
    }

    fn device_id(&self, msg: &MqttInMessage) -> String {
        let topic = &msg.topic;

        topic
            .strip_prefix(&self.base_topic)
            .unwrap_or(topic)
            .trim_matches('/')
            .to_owned()
    }

    fn get_events(
        &self,
        device_id: &str,
        channel: &Z2mChannel,
        payload: &str,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let result: Vec<IncomingData> = match channel {
            Z2mChannel::ClimateSensor(t, h) => {
                let payload: ClimateSensor = serde_json::from_str(payload)?;

                vec![
                    DataPoint::new(
                        ChannelValue::Temperature(t.clone(), DegreeCelsius(payload.temperature)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        ChannelValue::RelativeHumidity(h.clone(), Percent(payload.humidity)),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::ContactSensor(opened) => {
                let payload: ContactSensor = serde_json::from_str(payload)?;
                vec![
                    DataPoint::new(
                        ChannelValue::Opened(opened.clone(), !payload.contact),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::PowerPlug(power, energy) => {
                let payload: PowerPlug = serde_json::from_str(payload)?;
                vec![
                    DataPoint::new(
                        ChannelValue::CurrentPowerUsage(
                            power.clone(),
                            Watt(payload.current_power_w),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        ChannelValue::TotalEnergyConsumption(
                            energy.clone(),
                            KiloWattHours(payload.total_energy_kwh),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }
        };

        Ok(result)
    }
}

fn availability(friendly_name: &str, last_seen: DateTime) -> IncomingData {
    ItemAvailability {
        source: "Z2M".to_string(),
        item: friendly_name.to_string(),
        last_seen,
        marked_offline: false,
    }
    .into()
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ClimateSensor {
    temperature: f64,
    humidity: f64,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ContactSensor {
    contact: bool,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PowerPlug {
    #[serde(rename = "power")]
    current_power_w: f64,
    #[serde(rename = "energy")]
    total_energy_kwh: f64,
    last_seen: DateTime,
}
