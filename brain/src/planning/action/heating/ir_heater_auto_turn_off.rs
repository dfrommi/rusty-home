use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};
use support::t;

use crate::{
    planning::{action::Action, planner::ActionExecution},
    port::DataPointAccess,
};

#[derive(Debug, Clone)]
pub struct IrHeaterAutoTurnOff {
    execution: ActionExecution,
}

impl IrHeaterAutoTurnOff {
    pub fn new() -> Self {
        Self {
            execution: ActionExecution::from_start(
                "IrHeaterAutoTurnOff[Bedroom]",
                SetPower {
                    device: PowerToggle::InfraredHeater,
                    power_on: false,
                },
            ),
        }
    }
}

impl<API> Action<API> for IrHeaterAutoTurnOff
where
    API: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> anyhow::Result<bool> {
        let device = Powered::InfraredHeater;
        let current = api.current_data_point(device).await?;

        //on for at least 1 hour
        Ok(current.value && current.timestamp.elapsed() > t!(1 hours))
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}

impl std::fmt::Display for IrHeaterAutoTurnOff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IrHeaterAutoTurnOff[Bedroom]",)
    }
}
