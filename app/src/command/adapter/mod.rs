pub mod db;
mod homeassistant;
mod tasmota;
mod z2m;

use crate::command::Command;

pub use homeassistant::HomeAssistantCommandExecutor;
pub use tasmota::TasmotaCommandExecutor;
pub use z2m::Z2mCommandExecutor;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}
