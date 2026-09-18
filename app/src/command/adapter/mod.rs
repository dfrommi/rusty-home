pub mod db;
mod homeassistant;
mod lgtv;
pub mod nuki;
mod tasmota;
pub mod z2m;

pub use homeassistant::HomeAssistantCommandExecutor;
pub use lgtv::LgTvCommandExecutor;
pub use nuki::NukiCommandExecutor;
pub use tasmota::TasmotaCommandExecutor;
pub use z2m::Z2mCommandExecutor;

mod metrics {
    use crate::observability::system_metric_increment;

    #[derive(Clone, Copy, derive_more::Display)]
    pub enum CommandTargetSystem {
        #[display("TASMOTA")]
        Tasmota,
        #[display("Z2M")]
        Z2M,
        #[display("NUKI")]
        Nuki,
        #[display("HA")]
        HomeAssistant,
        #[allow(clippy::upper_case_acronyms)]
        #[display("LGTV")]
        LGTV,
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
