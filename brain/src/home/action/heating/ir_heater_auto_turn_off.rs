use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};
use support::t;

use crate::{home::action::Action, port::DataPointAccess};

use super::ActionExecution;

#[derive(Debug, Clone)]
pub struct IrHeaterAutoTurnOff;

impl IrHeaterAutoTurnOff {
    pub fn new() -> Self {
        Self {}
    }
}

impl<API> Action<API, SetPower> for IrHeaterAutoTurnOff
where
    API: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> anyhow::Result<bool> {
        let device = Powered::InfraredHeater;
        let current = api.current_data_point(device).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }

    fn execution(&self) -> ActionExecution<SetPower> {
        ActionExecution::trigger(
            self.to_string(),
            SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            },
        )
    }
}

impl std::fmt::Display for IrHeaterAutoTurnOff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IrHeaterAutoTurnOff[Bedroom]",)
    }
}
