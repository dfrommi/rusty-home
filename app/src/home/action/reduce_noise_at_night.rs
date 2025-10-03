use std::fmt::Display;

use crate::core::HomeApi;
use crate::core::time::DailyTimeRange;
use crate::home::command::{Command, PowerToggle};
use crate::t;

use crate::core::planner::SimpleAction;

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

impl SimpleAction for ReduceNoiseAtNight {
    fn command(&self) -> Command {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: false,
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, _: &HomeApi) -> anyhow::Result<bool> {
        Ok(self.range.contains(t!(now).time()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::time::Time;

    #[test]
    fn display_is_expected() {
        let range = DailyTimeRange::new(Time::at(22, 0).unwrap(), Time::at(6, 0).unwrap());
        assert_eq!(ReduceNoiseAtNight::new(range).to_string(), "ReduceNoiseAtNight");
    }
}
