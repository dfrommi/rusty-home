mod api;
mod command;
mod event;

use ::api::state::{
    CurrentPowerUsage, ExternalAutoControl, HeatingDemand, Opened, Powered, Presence,
    RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};

pub use api::HaRestClient;
pub use command::HaCommandExecutor;
pub use event::HaStateCollector;

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
