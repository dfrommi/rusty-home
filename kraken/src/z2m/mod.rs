use std::collections::HashMap;

use api::state::{
    ChannelValue, CurrentPowerUsage, Opened, RelativeHumidity, Temperature, TotalEnergyConsumption,
};
use support::{
    mqtt::MqttInMessage,
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent, Watt},
    DataPoint,
};

use crate::core::{IncomingData, IncomingDataProcessor, ItemAvailability};

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
}

pub struct Z2mStateCollector {
    base_topic: String,
    rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
    config: HashMap<String, Vec<Z2mChannel>>,
}

impl Z2mStateCollector {
    pub fn new(
        base_topic: String,
        rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
        config: &[(&str, Z2mChannel)],
    ) -> Self {
        let mut m: HashMap<String, Vec<Z2mChannel>> = HashMap::new();
        for (id, channel) in config {
            let id = id.to_string();
            m.entry(id).or_default().push(channel.clone());
        }

        let base_topic = if !base_topic.ends_with('/') {
            format!("{}/", base_topic)
        } else {
            base_topic
        };

        Self {
            base_topic,
            rx,
            config: m,
        }
    }
}

impl IncomingDataProcessor for Z2mStateCollector {
    async fn process(
        &mut self,
        sender: tokio::sync::mpsc::Sender<IncomingData>,
    ) -> anyhow::Result<()> {
        loop {
            let msg = match self.rx.recv().await {
                Some(msg) => msg,
                None => {
                    anyhow::bail!("Event receiver closed");
                }
            };

            for event in self.get_events(&msg) {
                if let Err(e) = sender.send(event.clone()).await {
                    tracing::error!("Error sending event {:?}: {:?}", event, e);
                }
            }
        }
    }
}

impl Z2mStateCollector {
    fn get_friendly_name(&self, topic: &str) -> String {
        topic
            .strip_prefix(&self.base_topic)
            .unwrap_or(topic)
            .to_owned()
    }

    fn get_events(&self, msg: &MqttInMessage) -> Vec<IncomingData> {
        let friendly_name = self.get_friendly_name(&msg.topic);
        let channels = match self.config.get(&friendly_name) {
            Some(channels) => channels,
            None => return vec![],
        };

        channels
            .iter()
            .flat_map(
                |c| match to_data_events(c.clone(), &msg.payload, &friendly_name) {
                    Ok(events) => events,
                    Err(e) => {
                        tracing::error!(
                            "Error parsing MQTT message: {} for channel {:?}: {:?}",
                            msg.topic,
                            c,
                            e
                        );
                        vec![]
                    }
                },
            )
            .collect()
    }
}

fn to_data_events(
    channel: Z2mChannel,
    payload: &str,
    friendly_name: &str,
) -> anyhow::Result<Vec<IncomingData>> {
    let result: Vec<IncomingData> = match channel {
        Z2mChannel::ClimateSensor(t, h) => {
            let payload: ClimateSensor = serde_json::from_str(payload)?;

            vec![
                DataPoint::new(
                    ChannelValue::Temperature(t, DegreeCelsius(payload.temperature)),
                    payload.last_seen,
                )
                .into(),
                DataPoint::new(
                    ChannelValue::RelativeHumidity(h, Percent(payload.humidity)),
                    payload.last_seen,
                )
                .into(),
                availability(friendly_name, payload.last_seen),
            ]
        }

        Z2mChannel::ContactSensor(opened) => {
            let payload: ContactSensor = serde_json::from_str(payload)?;
            vec![
                DataPoint::new(
                    ChannelValue::Opened(opened, !payload.contact),
                    payload.last_seen,
                )
                .into(),
                availability(friendly_name, payload.last_seen),
            ]
        }

        Z2mChannel::PowerPlug(power, energy) => {
            let payload: PowerPlug = serde_json::from_str(payload)?;
            vec![
                DataPoint::new(
                    ChannelValue::CurrentPowerUsage(power, Watt(payload.current_power_w)),
                    payload.last_seen,
                )
                .into(),
                DataPoint::new(
                    ChannelValue::TotalEnergyConsumption(
                        energy,
                        KiloWattHours(payload.total_energy_kwh),
                    ),
                    payload.last_seen,
                )
                .into(),
                availability(friendly_name, payload.last_seen),
            ]
        }
    };

    Ok(result)
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
