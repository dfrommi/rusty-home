use crate::core::HomeApi;
use crate::home::command::{Command, PowerToggle};
use crate::home::state::Powered;
use crate::t;

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

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let device = Powered::InfraredHeater;
        let current = device.current_data_point(api).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_expected() {
        assert_eq!(IrHeaterAutoTurnOff::new().to_string(), "IrHeaterAutoTurnOff[Bedroom]");
    }
}
