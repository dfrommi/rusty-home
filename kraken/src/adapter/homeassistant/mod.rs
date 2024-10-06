mod command;
mod config;
mod event;

use api::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Powered, RelativeHumidity, Temperature,
    TotalEnergyConsumption,
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
    HeatingDemand(HeatingDemand),
}

enum HomeAssistantService {
    SwitchTurnOn,
    SwitchTurnOff,
}

struct HaCommandEntity<'a> {
    pub id: &'a str,
    pub service: HomeAssistantService,
}
