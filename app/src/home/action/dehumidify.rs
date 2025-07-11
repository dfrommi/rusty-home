use std::fmt::Display;

use crate::home::command::{Command, PowerToggle};
use anyhow::Result;

use crate::{core::planner::SimpleAction, home::state::RiskOfMould};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub struct Dehumidify;

impl Dehumidify {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for Dehumidify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dehumidify")
    }
}

impl SimpleAction for Dehumidify {
    fn command(&self) -> Command {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> Result<bool> {
        RiskOfMould::Bathroom.current(api).await
    }
}
