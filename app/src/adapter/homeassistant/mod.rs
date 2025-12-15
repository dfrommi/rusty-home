mod client;
mod config;
mod incoming;
mod outgoing;

use client::HaHttpClient;
use client::HaMqttClient;
use incoming::HaIncomingDataSource;
use outgoing::HaCommandExecutor;

use crate::adapter::command::CommandExecutor;
use crate::device_state::DeviceStateClient;
use crate::device_state::{FanActivity, LightLevel, PowerAvailable, Presence, RelativeHumidity, Temperature};
use std::collections::HashMap;

use crate::core::time::DateTime;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::Infrastructure;
use crate::core::DeviceConfig;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HomeAssitant {
    pub topic_event: String,
    pub url: String,
    pub token: String,
}

impl HomeAssitant {
    pub fn new_command_executor(&self, device_client: DeviceStateClient) -> impl CommandExecutor + use<> {
        let http_client =
            HaHttpClient::new(&self.url, &self.token).expect("Error initializing Home Assistant REST client");
        HaCommandExecutor::new(http_client, device_client, &config::default_ha_command_config())
    }

    pub async fn new_incoming_data_source(&self, infrastructure: &mut Infrastructure) -> HaIncomingDataSource {
        let mqtt = &mut infrastructure.mqtt_client;
        let config = DeviceConfig::new(&config::default_ha_state_config());
        let rx = mqtt
            .subscribe(self.topic_event.clone())
            .await
            .expect("Error subscribing to MQTT topic");

        let mqtt_client = HaMqttClient::new(rx);
        let http_client = HaHttpClient::new(&self.url, &self.token).expect("Error creating HA HTTP client");

        HaIncomingDataSource::new(http_client, mqtt_client, config)
    }
}

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

#[derive(Debug, Clone)]
enum HaServiceTarget {
    LightTurnOnOff(&'static str),
    PushNotification(&'static str),
    LgWebosSmartTv(&'static str),
    WindcalmFanSpeed(&'static str),
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
