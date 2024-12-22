use api::{
    command::{Command, PowerToggle, SetPower},
    state::Powered,
};
use support::t;

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    port::DataPointAccess,
};

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

impl CommandAction for IrHeaterAutoTurnOff {
    fn command(&self) -> Command {
        Command::SetPower(SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: false,
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<API> ConditionalAction<API> for IrHeaterAutoTurnOff
where
    API: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> anyhow::Result<bool> {
        let device = Powered::InfraredHeater;
        let current = api.current_data_point(device).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }
}
