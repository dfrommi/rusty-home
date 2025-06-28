mod client;
mod command;
mod state;

pub use client::HaHttpClient;
pub use client::HaMqttClient;
pub use command::HaCommandExecutor;

use ::api::state::{
    ExternalAutoControl, HeatingDemand, Powered, Presence, RelativeHumidity, SetPoint, Temperature,
};
use api::state::FanActivity;
use state::HaIncomingDataProcessor;

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use support::time::DateTime;

use crate::core::IncomingDataProcessor;

pub fn new_incoming_data_processor(
    client: HaHttpClient,
    mqtt_client: HaMqttClient,
    config: &[(&str, HaChannel)],
) -> anyhow::Result<impl IncomingDataProcessor + use<>> {
    let collector = HaIncomingDataProcessor::new(client, mqtt_client, config);
    Ok(collector)
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
