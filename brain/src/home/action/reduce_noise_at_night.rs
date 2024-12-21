use std::fmt::Display;

use api::{
    command::{PowerToggle, SetPower},
    state::Powered,
};
use support::{t, time::DailyTimeRange};

use super::{Action, ActionExecution, DataPointAccess};

#[derive(Debug, Clone)]
pub struct ReduceNoiseAtNight {
    range: DailyTimeRange,
}

impl ReduceNoiseAtNight {
    pub fn new(range: DailyTimeRange) -> Self {
        Self { range }
    }
}

impl<T> Action<T, SetPower> for ReduceNoiseAtNight
where
    T: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, _: &T) -> anyhow::Result<bool> {
        Ok(self.range.contains(t!(now).time()))
    }

    fn execution(&self) -> ActionExecution<SetPower> {
        ActionExecution::trigger(
            self.to_string(),
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            },
        )
    }
}

impl Display for ReduceNoiseAtNight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReduceNoiseAtNight")
    }
}
