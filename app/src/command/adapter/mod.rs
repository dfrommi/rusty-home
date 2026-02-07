pub mod db;
mod homeassistant;
mod tasmota;
pub mod z2m;

use crate::command::Command;

pub use homeassistant::HomeAssistantCommandExecutor;
pub use tasmota::TasmotaCommandExecutor;
pub use z2m::Z2mCommandExecutor;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

mod metrics {
    use crate::observability::system_metric_increment;

    #[derive(Clone, Copy, derive_more::Display)]
    pub enum CommandTargetSystem {
        #[display("TASMOTA")]
        Tasmota,
        #[display("Z2M")]
        Z2M,
        #[display("HA")]
        HomeAssistant,
    }

    pub enum CommandMetric {
        Executed {
            device_id: String,
            system: CommandTargetSystem,
        },
    }

    impl CommandMetric {
        pub fn record(&self) {
            match self {
                CommandMetric::Executed { device_id, system } => {
                    let system = system.to_string();
                    system_metric_increment(
                        "command_executed",
                        &[("device_id", device_id.as_str()), ("system", system.as_str())],
                    );
                }
            }
        }
    }
}
