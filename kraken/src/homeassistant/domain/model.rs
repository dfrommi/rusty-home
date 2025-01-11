use ::api::state::{
    CurrentPowerUsage, ExternalAutoControl, HeatingDemand, Opened, Powered, Presence,
    RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};
use api::trigger::RemoteTarget;

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
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
    ButtonPress(RemoteTarget),
}

#[derive(Debug, Clone)]
pub enum HaServiceTarget {
    SwitchTurnOnOff(&'static str),
    LightTurnOnOff(&'static str),
    ClimateControl(&'static str),
    PushNotification(&'static str),
    LgWebosSmartTv(&'static str),
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
