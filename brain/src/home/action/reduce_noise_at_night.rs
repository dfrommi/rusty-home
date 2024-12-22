use std::fmt::Display;

use api::{
    command::{Command, PowerToggle, SetPower},
    state::Powered,
};
use support::{t, time::DailyTimeRange};

use crate::core::planner::{CommandAction, ConditionalAction};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub struct ReduceNoiseAtNight {
    range: DailyTimeRange,
}

impl ReduceNoiseAtNight {
    pub fn new(range: DailyTimeRange) -> Self {
        Self { range }
    }
}

impl Display for ReduceNoiseAtNight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReduceNoiseAtNight")
    }
}

impl CommandAction for ReduceNoiseAtNight {
    fn command(&self) -> Command {
        Command::SetPower(SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: false,
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for ReduceNoiseAtNight
where
    T: DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, _: &T) -> anyhow::Result<bool> {
        Ok(self.range.contains(t!(now).time()))
    }
}
