use std::fmt::Display;

use crate::core::HomeApi;
use crate::home::command::{Command, PowerToggle};
use crate::t;

use crate::core::planner::SimpleAction;

#[derive(Debug, Clone)]
pub enum ReduceNoiseAtNight {
    Dehumidifier,
}

impl Display for ReduceNoiseAtNight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReduceNoiseAtNight")
    }
}

impl SimpleAction for ReduceNoiseAtNight {
    fn command(&self) -> Command {
        match self {
            ReduceNoiseAtNight::Dehumidifier => Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            },
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, _: &HomeApi) -> anyhow::Result<bool> {
        match self {
            ReduceNoiseAtNight::Dehumidifier => Ok(t!(22:30 - 12:00).contains(t!(now).time())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_expected() {
        assert_eq!(ReduceNoiseAtNight::Dehumidifier.to_string(), "ReduceNoiseAtNight");
    }
}
