use super::TasmotaCommandTarget;
use crate::command::{CommandTarget, PowerToggle};

pub fn default_tasmota_command_config() -> Vec<(CommandTarget, TasmotaCommandTarget)> {
    vec![
        (
            CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
            TasmotaCommandTarget::PowerSwitch("dehumidifier"),
        ),
        (
            CommandTarget::SetPower {
                device: PowerToggle::InfraredHeater,
            },
            TasmotaCommandTarget::PowerSwitch("irheater"),
        ),
    ]
}
