mod client;
mod config;
mod incoming;
mod outgoing;

use client::HaHttpClient;
use client::HaMqttClient;
use incoming::HaIncomingDataSource;
use outgoing::HaCommandExecutor;

use crate::home::state::{FanActivity, HeatingDemand, Powered, Presence, RelativeHumidity, SetPoint, Temperature};
use infrastructure::Mqtt;

use std::collections::HashMap;

use crate::core::time::DateTime;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::Infrastructure;
use crate::core::CommandExecutor;
use crate::core::DeviceConfig;
use crate::core::process_incoming_data_source;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HomeAssitant {
    pub topic_event: String,
    pub url: String,
    pub token: String,
}

impl HomeAssitant {
    pub async fn new_incoming_data_processor(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl Future<Output = ()> + use<> {
        let ds = self.new_incoming_data_source(&mut infrastructure.mqtt_client).await;

        let api = infrastructure.api.clone();
        async move { process_incoming_data_source("HomeAssitant", ds, &api).await }
    }

    pub fn new_command_executor(&self, infrastructure: &Infrastructure) -> impl CommandExecutor + use<> {
        let http_client =
            HaHttpClient::new(&self.url, &self.token).expect("Error initializing Home Assistant REST client");
        HaCommandExecutor::new(http_client, infrastructure.api.clone(), &config::default_ha_command_config())
    }

    async fn new_incoming_data_source(&self, mqtt: &mut Mqtt) -> HaIncomingDataSource {
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
enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Powered(Powered),
    SetPoint(SetPoint),
    HeatingDemand(HeatingDemand),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
    WindcalmFanSpeed(FanActivity),
}

#[derive(Debug, Clone)]
enum HaServiceTarget {
    LightTurnOnOff(&'static str),
    PushNotification(&'static str),
    LgWebosSmartTv(&'static str),
    WindcalmFanSpeed(&'static str),
}

#[derive(Deserialize, Debug)]
struct StateChangedEvent {
    pub entity_id: String,
    pub state: StateValue,
    pub last_changed: DateTime,
    pub last_updated: DateTime,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug)]
enum StateValue {
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
