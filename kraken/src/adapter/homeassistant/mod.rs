mod command;
mod event;

use api::state::{
    CurrentPowerUsage, ExternalAutoControl, HeatingDemand, Opened, Powered, Presence,
    RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};

use chrono::{DateTime, Utc};
pub use command::HaCommandExecutor;
pub use event::HaStateCollector;
use support::unit::DegreeCelsius;

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
pub enum HaService {
    SwitchTurnOnOff {
        id: String,
        power_on: bool,
    },
    LightTurnOnOff {
        id: String,
        power_on: bool,
    },
    TadoSetClimateTimer {
        id: String,
        temperature: DegreeCelsius,
        until: DateTime<Utc>,
    },
    ClimateSetHvacMode {
        id: String,
        mode: HaClimateHvacMode,
    },
}

#[derive(Debug, Clone)]
pub enum HaClimateHvacMode {
    Off,
    Auto,
}
