use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};
use support::t;

use crate::{planning::action::Action, port::DataPointAccess};

#[derive(Debug, Clone)]
pub enum IrHeaterAutoTurnOff {
    Bedroom,
}

impl<API> Action<API> for IrHeaterAutoTurnOff
where
    API: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> anyhow::Result<bool> {
        let device = match self {
            IrHeaterAutoTurnOff::Bedroom => Powered::InfraredHeater,
        };

        let current = api.current_data_point(device).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }

    fn start_command(&self) -> Option<api::command::Command> {
        Some(
            SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<api::command::Command> {
        None
    }
}

impl std::fmt::Display for IrHeaterAutoTurnOff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IrHeaterAutoTurnOff[{}]",
            match self {
                IrHeaterAutoTurnOff::Bedroom => "Bedroom",
            }
        )
    }
}
