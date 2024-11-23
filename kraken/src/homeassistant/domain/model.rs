use ::api::state::{
    CurrentPowerUsage, ExternalAutoControl, HeatingDemand, Opened, Powered, Presence,
    RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use support::time::DateTime;

#[derive(Debug, Clone)]
pub enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Opened(Opened),
    Powered(Powered),
    CurrentPowerUsage(CurrentPowerUsage),
    TotalEnergyConsumption(TotalEnergyConsumption),
    SetPoint(SetPoint),
    HeatingDemand(HeatingDemand),
    ClimateAutoMode(ExternalAutoControl),
    PresenceFromLeakSensor(Presence),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
}

#[derive(Debug, Clone)]
pub enum HaServiceTarget {
    SwitchTurnOnOff(String),
    LightTurnOnOff(String),
    ClimateControl(String),
}

//TODO is Serialize necessary?
#[derive(Deserialize, Serialize, Debug)]
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

//TODO can deserialization of even be in the adapter?
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

impl Serialize for StateValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StateValue::Available(v) => serializer.serialize_str(v),
            StateValue::Unavailable => serializer.serialize_str("unavailable"),
        }
    }
}
