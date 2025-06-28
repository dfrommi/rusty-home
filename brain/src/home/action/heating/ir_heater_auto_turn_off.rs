use api::{
    command::{Command, PowerToggle},
    state::Powered,
};
use support::t;

use crate::{core::planner::SimpleAction, port::DataPointAccess};

#[derive(Debug, Clone)]
pub struct IrHeaterAutoTurnOff;

impl IrHeaterAutoTurnOff {
    pub fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Display for IrHeaterAutoTurnOff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IrHeaterAutoTurnOff[Bedroom]",)
    }
}

impl SimpleAction for IrHeaterAutoTurnOff {
    fn command(&self) -> Command {
        Command::SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: false,
        }
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::Database) -> anyhow::Result<bool> {
        let device = Powered::InfraredHeater;
        let current = api.current_data_point(device).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }
}
