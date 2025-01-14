mod command;
mod event;

use api::state::{CurrentPowerUsage, Powered, TotalEnergyConsumption};

pub use command::TasmotaCommandExecutor;
pub use event::TasmotaMqttParser;

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    EnergyMeter(CurrentPowerUsage, TotalEnergyConsumption),
    PowerToggle(Powered),
}

#[derive(Debug, Clone)]
pub enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}
