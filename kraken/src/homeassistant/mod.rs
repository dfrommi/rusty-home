mod client;
mod command;
mod state;

pub use client::HaHttpClient;
pub use client::HaMqttClient;
pub use command::HaCommandExecutor;
pub use state::HaIncomingDataSource;

use ::api::state::{
    ExternalAutoControl, HeatingDemand, Powered, Presence, RelativeHumidity, SetPoint, Temperature,
};
use api::state::FanActivity;
use infrastructure::Mqtt;

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use support::time::DateTime;

use crate::core::DeviceConfig;

pub async fn new_incoming_data_source(
    url: &str,
    token: &str,
    topic: &str,
    config: &[(&str, HaChannel)],
    mqtt: &mut Mqtt,
) -> HaIncomingDataSource {
    let config = DeviceConfig::new(config);
    let rx = mqtt
        .subscribe(topic)
        .await
        .expect("Error subscribing to MQTT topic");

    let mqtt_client = HaMqttClient::new(rx);
    let http_client = HaHttpClient::new(url, token).expect("Error creating HA HTTP client");

    HaIncomingDataSource::new(http_client, mqtt_client, config)
}

#[derive(Debug, Clone)]
pub enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Powered(Powered),
    SetPoint(SetPoint),
    HeatingDemand(HeatingDemand),
    ClimateAutoMode(ExternalAutoControl),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
    WindcalmFanSpeed(FanActivity),
}

#[derive(Debug, Clone)]
pub enum HaServiceTarget {
    LightTurnOnOff(&'static str),
    ClimateControl(&'static str),
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
