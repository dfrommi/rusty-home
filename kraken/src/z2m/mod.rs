use api::{
    state::{
        ChannelValue, CurrentPowerUsage, Opened, Presence, RelativeHumidity, Temperature,
        TotalEnergyConsumption,
    },
    trigger::{ButtonPress, Remote, RemoteTarget, UserTrigger},
};
use infrastructure::{Mqtt, MqttInMessage};
use support::{
    DataPoint,
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent, Watt},
};
use tokio::sync::mpsc;

use crate::core::{DeviceConfig, IncomingData, IncomingDataSource, ItemAvailability};

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
    PresenceFromLeakSensor(Presence),
    RemoteClick(RemoteTarget),
}

pub async fn new_incoming_data_source(
    base_topic: &str,
    config: &[(&str, Z2mChannel)],
    mqtt: &mut Mqtt,
) -> Z2mIncomingDataSource {
    let config = DeviceConfig::new(config);
    let rx = mqtt
        .subscribe(format!("{}/#", base_topic))
        .await
        .expect("Error subscribing to MQTT topic");

    Z2mIncomingDataSource::new(base_topic.to_string(), config, rx)
}

pub struct Z2mIncomingDataSource {
    base_topic: String,
    device_config: DeviceConfig<Z2mChannel>,
    mqtt_receiver: mpsc::Receiver<MqttInMessage>,
}

impl Z2mIncomingDataSource {
    pub fn new(
        base_topic: String,
        config: DeviceConfig<Z2mChannel>,
        mqtt_rx: mpsc::Receiver<MqttInMessage>,
    ) -> Self {
        Self {
            base_topic: base_topic.trim_matches('/').to_owned(),
            device_config: config,
            mqtt_receiver: mqtt_rx,
        }
    }
}

impl IncomingDataSource<MqttInMessage, Z2mChannel> for Z2mIncomingDataSource {
    async fn recv(&mut self) -> Option<MqttInMessage> {
        self.mqtt_receiver.recv().await
    }

    fn device_id(&self, msg: &MqttInMessage) -> Option<String> {
        let topic = &msg.topic;

        topic
            .strip_prefix(&self.base_topic)
            .map(|topic| topic.trim_matches('/').to_owned())
    }

    fn get_channels(&self, device_id: &str) -> &[Z2mChannel] {
        self.device_config.get(device_id)
    }

    fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &Z2mChannel,
        msg: &MqttInMessage,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let result: Vec<IncomingData> = match channel {
            Z2mChannel::ClimateSensor(t, h) => {
                let payload: ClimateSensor = serde_json::from_str(&msg.payload)?;

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
                let payload: ContactSensor = serde_json::from_str(&msg.payload)?;
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
                let payload: PowerPlug = serde_json::from_str(&msg.payload)?;
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

            Z2mChannel::PresenceFromLeakSensor(presence) => {
                let payload: WaterLeakSensor = serde_json::from_str(&msg.payload)?;
                vec![
                    DataPoint::new(
                        ChannelValue::Presence(presence.clone(), payload.water_leak),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::RemoteClick(target) => {
                let payload: RemoteControl = serde_json::from_str(&msg.payload)?;
                let mut events = vec![availability(device_id, payload.last_seen)];

                let button_press = match payload.action.as_deref() {
                    Some("on") => Some(ButtonPress::TopSingle),
                    Some("off") => Some(ButtonPress::BottomSingle),
                    _ => None,
                };

                if let Some(button_press) = button_press {
                    events.push(
                        UserTrigger::Remote(match target {
                            RemoteTarget::BedroomDoor => Remote::BedroomDoor(button_press),
                        })
                        .into(),
                    );
                }

                events
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

#[derive(Debug, Clone, serde::Deserialize)]
struct WaterLeakSensor {
    water_leak: bool,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RemoteControl {
    action: Option<String>,
    last_seen: DateTime,
}
