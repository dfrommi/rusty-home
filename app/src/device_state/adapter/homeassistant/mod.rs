mod config;

use infrastructure::{Mqtt, MqttInMessage};
use serde::Deserialize;

use anyhow::Context;
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{
    DeviceAvailability, FanActivity, LightLevel, PowerAvailable, Presence, RelativeHumidity, Temperature,
};
use std::collections::HashMap;

use crate::core::time::DateTime;
use serde::Deserializer;
use serde_json::Value;

use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, FanAirflow, Lux, Percent};
use crate::device_state::DeviceStateValue;

use crate::core::DeviceConfig;

#[derive(Debug, Clone)]
pub enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Powered(PowerAvailable),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
    PresenceFromFP2(Presence),
    WindcalmFanSpeed(FanActivity),
    LightLevel(LightLevel),
}

#[derive(Deserialize, Debug)]
pub struct StateChangedEvent {
    pub entity_id: String,
    pub state: StateValue,
    pub last_changed: DateTime,
    pub last_updated: DateTime,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug)]
pub enum StateValue {
    Available(String),
    Unavailable,
}

//TODO can deserialization of event be in the adapter?
impl<'de> Deserialize<'de> for StateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "unavailable" => Ok(StateValue::Unavailable),
            _ => Ok(StateValue::Available(value)),
        }
    }
}

pub struct HomeAssistantIncomingDataSource {
    client: HaHttpClient,
    listener: HaMqttClient,
    config: DeviceConfig<HaChannel>,
    initial_load: Option<Vec<StateChangedEvent>>,
}

impl HomeAssistantIncomingDataSource {
    pub async fn new(mqtt: &mut Mqtt, event_topic: &str, url: &str, token: &str) -> Self {
        let config = DeviceConfig::new(&config::default_ha_state_config());
        let rx = mqtt
            .subscribe(event_topic)
            .await
            .expect("Error subscribing to MQTT topic");

        let mqtt_client = HaMqttClient::new(rx);
        let http_client = HaHttpClient::new(url, token).expect("Error creating HA HTTP client");

        Self {
            client: http_client,
            listener: mqtt_client,
            config,
            initial_load: None,
        }
    }
}

impl IncomingDataSource<StateChangedEvent, HaChannel> for HomeAssistantIncomingDataSource {
    fn ds_name(&self) -> &str {
        "HomeAssistant"
    }

    async fn recv(&mut self) -> Option<StateChangedEvent> {
        if self.initial_load.is_none() {
            self.initial_load = match self.client.get_current_state().await {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::error!("Error loading initial state for HA: {:?}", e);
                    Some(vec![])
                }
            };
        }

        match &mut self.initial_load {
            Some(data) if !data.is_empty() => data.pop(),
            _ => self.listener.recv().await,
        }
    }

    fn device_id(&self, msg: &StateChangedEvent) -> Option<String> {
        Some(msg.entity_id.clone())
    }

    fn get_channels(&self, device_id: &str) -> &[HaChannel] {
        self.config.get(device_id)
    }

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &HaChannel,
        msg: &StateChangedEvent,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let mut result = match &msg.state {
            StateValue::Available(state_value) => {
                tracing::info!("Received supported event {}", device_id);

                let dp_result =
                    to_persistent_data_point(channel.clone(), state_value, &msg.attributes, msg.last_changed);

                match dp_result {
                    Ok(Some(dp)) => vec![dp],
                    Ok(None) => vec![],
                    Err(e) => {
                        tracing::error!("Error processing homeassistant event of {}: {:?}", device_id, e);
                        vec![]
                    }
                }
            }
            _ => {
                tracing::warn!("Value of {} is not available", device_id);
                vec![]
            }
        };

        result.push(to_item_availability(msg));

        Ok(result)
    }
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    _attributes: &HashMap<String, serde_json::Value>,
    timestamp: DateTime,
) -> anyhow::Result<Option<IncomingData>> {
    let dp: Option<IncomingData> = match channel {
        HaChannel::Temperature(channel) => Some(
            DataPoint::new(
                DeviceStateValue::Temperature(channel, DegreeCelsius(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::RelativeHumidity(channel) => Some(
            DataPoint::new(
                DeviceStateValue::RelativeHumidity(channel, Percent(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::Powered(channel) => {
            Some(DataPoint::new(DeviceStateValue::PowerAvailable(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::PresenceFromEsp(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::PresenceFromDeviceTracker(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "home"), timestamp).into())
        }
        HaChannel::LightLevel(channel) => {
            Some(DataPoint::new(DeviceStateValue::LightLevel(channel, Lux(ha_value.parse()?)), timestamp).into())
        }
        HaChannel::PresenceFromFP2(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::WindcalmFanSpeed(channel) => {
            //Fan-Speed updates are extremely unreliable at the moment. Only use Off as a reset
            //trigger, otherwise assume command worked an directly set state from command
            //processing, assuming everything is done via the smart home
            /*
            last_seent fan_speed = match attributes.get("percentage").and_then(|v| v.as_f64()) {
                Some(1.0) => FanSpeed::Silent,
                Some(f_value) if f_value <= 20.0 => FanSpeed::Low,
                Some(f_value) if f_value <= 40.0 => FanSpeed::Medium,
                Some(f_value) if f_value <= 60.0 => FanSpeed::High,
                Some(_) => FanSpeed::Turbo,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            let airflow = if ha_value == "off" {
                FanAirflow::Off
            } else if attributes.get("direction").and_then(|v| v.as_str()) == Some("reverse") {
                FanAirflow::Reverse(fan_speed)
            } else {
                FanAirflow::Forward(fan_speed)
            };

            Some(DataPoint::new(ChannelValue::FanActivity(channel, airflow), timestamp).into())
            */

            if ha_value == "off" {
                Some(DataPoint::new(DeviceStateValue::FanActivity(channel, FanAirflow::Off), timestamp).into())
            } else {
                None
            }
        }
    };

    Ok(dp)
}

fn to_item_availability(new_state: &StateChangedEvent) -> IncomingData {
    let entity_id: &str = &new_state.entity_id;

    DeviceAvailability {
        source: "HA".to_string(),
        device_id: entity_id.to_string(),
        last_seen: new_state.last_updated,
        marked_offline: matches!(new_state.state, StateValue::Unavailable),
    }
    .into()
}

#[derive(Debug, Clone)]
pub struct HaHttpClient {
    client: ClientWithMiddleware,
    base_url: String,
}

impl HaHttpClient {
    pub fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(Some(token.to_owned())).new_tracing_client()?;

        Ok(Self {
            client,
            base_url: url.to_owned(),
        })
    }
}

impl HaHttpClient {
    pub async fn get_current_state(&self) -> anyhow::Result<Vec<StateChangedEvent>> {
        let response = self.client.get(format!("{}/api/states", self.base_url)).send().await?;

        response
            .json::<Vec<StateChangedEvent>>()
            .await
            .context("Error getting all states")
    }
}

pub struct HaMqttClient {
    state_rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
}

impl HaMqttClient {
    pub fn new(rx: tokio::sync::mpsc::Receiver<MqttInMessage>) -> Self {
        Self { state_rx: rx }
    }
}

impl HaMqttClient {
    pub async fn recv(&mut self) -> Option<StateChangedEvent> {
        match self.state_rx.recv().await {
            Some(msg) => {
                match serde_json::from_str::<HaEvent>(&msg.payload) {
                    Ok(HaEvent::StateChanged { new_state: event, .. }) => Some(event),
                    Ok(HaEvent::Unknown(_)) => {
                        tracing::trace!("Received unsupported event: {:?}", msg.payload);
                        None
                    }

                    //json parsing error
                    Err(e) => {
                        tracing::error!("Error parsing MQTT message: {}", e);
                        None
                    }
                }
            }

            None => {
                tracing::error!("Error parsing MQTT message: channel closed");
                None
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "event_type", content = "event_data")]
#[allow(dead_code)]
pub enum HaEvent {
    #[serde(rename = "state_changed")]
    StateChanged {
        entity_id: String,
        new_state: StateChangedEvent,
    },

    #[serde(untagged)]
    Unknown(serde_json::Value),
}
