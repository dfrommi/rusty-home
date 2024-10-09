mod command;
mod event;

use api::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Powered, Presence, RelativeHumidity, SetPoint,
    Temperature, TotalEnergyConsumption, UserControlled,
};

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
    UserControlledOverlay(UserControlled),
    PresenceFromLeakSensor(Presence),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
}

#[derive(Debug, Clone)]
pub enum HaService {
    SwitchTurnOn { id: String },
    SwitchTurnOff { id: String },
    LightTurnOn { id: String },
    LightTurnOff { id: String },
}
