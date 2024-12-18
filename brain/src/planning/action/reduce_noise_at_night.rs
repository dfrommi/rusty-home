use std::fmt::Display;

use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};
use support::{t, time::DailyTimeRange};

use crate::planning::planner::ActionExecution;

use super::{Action, DataPointAccess};

#[derive(Debug, Clone)]
pub struct ReduceNoiseAtNight {
    range: DailyTimeRange,
    execution: ActionExecution,
}

impl ReduceNoiseAtNight {
    pub fn new(range: DailyTimeRange) -> Self {
        Self {
            range,
            execution: ActionExecution::from_start(
                "ReduceNoiseAtNight",
                SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on: false,
                },
            ),
        }
    }
}

impl<T> Action<T> for ReduceNoiseAtNight
where
    T: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, _: &T) -> anyhow::Result<bool> {
        Ok(self.range.contains(t!(now).time()))
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}

impl Display for ReduceNoiseAtNight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReduceNoiseAtNight")
    }
}
