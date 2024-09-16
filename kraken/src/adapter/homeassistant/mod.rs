mod command;
mod config;
mod event;

pub use command::to_command_payload;
pub use event::{init, to_smart_home_event};

use api::state::{
    CurrentPowerUsage, Opened, Powered, RelativeHumidity, Temperature, TotalEnergyConsumption,
};

struct HaSensorEntity<'a> {
    pub id: &'a str,
    pub channel: HaChannel,
}

#[derive(Debug, Clone)]
enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Opened(Opened),
    Powered(Powered),
    CurrentPowerUsage(CurrentPowerUsage),
    TotalEnergyConsumption(TotalEnergyConsumption),
}

enum HomeAssistantService {
    SwitchTurnOn,
    SwitchTurnOff,
}

struct HaCommandEntity<'a> {
    pub id: &'a str,
    pub service: HomeAssistantService,
}
