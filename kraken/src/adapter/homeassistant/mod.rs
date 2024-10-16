mod command;
mod event;

use api::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Powered, Presence, RelativeHumidity, SetPoint,
    Temperature, TotalEnergyConsumption, UserControlled,
};

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
    UserControlledOverlay(UserControlled),
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
    ClimateSetTemperature {
        id: String,
        temperature: Option<DegreeCelsius>,
    },
}
