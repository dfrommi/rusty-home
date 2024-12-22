use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{Command, PowerToggle, SetPower},
    state::Powered,
};

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    home::state::RiskOfMould,
};

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

impl CommandAction for Dehumidify {
    fn command(&self) -> Command {
        Command::SetPower(SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for Dehumidify
where
    T: DataPointAccess<RiskOfMould> + DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(RiskOfMould::Bathroom).await
    }
}
