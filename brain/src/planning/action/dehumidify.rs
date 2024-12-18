use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};

use crate::planning::planner::ActionExecution;

use super::{Action, DataPointAccess, RiskOfMould};

#[derive(Debug, Clone)]
pub struct Dehumidify {
    execution: ActionExecution,
}

impl Dehumidify {
    pub fn new() -> Self {
        Self {
            execution: ActionExecution::from_start_and_stop(
                "Dehumidify",
                SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on: true,
                },
                SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on: false,
                },
            ),
        }
    }
}

impl<T> Action<T> for Dehumidify
where
    T: DataPointAccess<RiskOfMould> + DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(RiskOfMould::Bathroom).await
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}

impl Display for Dehumidify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dehumidify")
    }
}
