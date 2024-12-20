use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};

use crate::home::state::RiskOfMould;

use super::{Action, ActionExecution, DataPointAccess};

#[derive(Debug, Clone)]
pub struct Dehumidify;

impl Dehumidify {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T> Action<T, SetPower> for Dehumidify
where
    T: DataPointAccess<RiskOfMould> + DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(RiskOfMould::Bathroom).await
    }

    fn execution(&self) -> ActionExecution<SetPower> {
        ActionExecution::from_start_and_stop(
            self.to_string(),
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            },
        )
    }
}

impl Display for Dehumidify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dehumidify")
    }
}
