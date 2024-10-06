mod command;
mod config;
mod event;

use api::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Powered, Presence, RelativeHumidity, SetPoint,
    Temperature, TotalEnergyConsumption,
};

pub use command::HaCommandExecutor;
pub use event::HaStateCollector;

#[derive(Debug, Clone)]
enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Opened(Opened),
    Powered(Powered),
    CurrentPowerUsage(CurrentPowerUsage),
    TotalEnergyConsumption(TotalEnergyConsumption),
    SetPoint(SetPoint),
    HeatingDemand(HeatingDemand),
    PresenceFromLeakSensor(Presence),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
}

enum HomeAssistantService {
    SwitchTurnOn,
    SwitchTurnOff,
}

struct HaCommandEntity<'a> {
    pub id: &'a str,
    pub service: HomeAssistantService,
}
