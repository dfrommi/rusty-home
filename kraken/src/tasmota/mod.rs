mod command;
mod event;

use api::state::{CurrentPowerUsage, TotalEnergyConsumption};

pub use command::TasmotaCommandExecutor;
pub use event::TasmotaMqttParser;

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
}

#[derive(Debug, Clone)]
pub enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}
