use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{Command, PowerToggle, SetPower},
    state::Powered,
};

use super::{Action, DataPointAccess, RiskOfMould};

#[derive(Debug, Clone)]
pub struct Dehumidify;

impl<T: DataPointAccess<RiskOfMould> + DataPointAccess<Powered>> Action<T> for Dehumidify {
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(RiskOfMould::Bathroom).await
    }

    async fn is_running(&self, api: &T) -> Result<bool> {
        api.current(Powered::Dehumidifier).await
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<Command> {
        Some(
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            }
            .into(),
        )
    }
}

impl Display for Dehumidify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dehumidify")
    }
}
