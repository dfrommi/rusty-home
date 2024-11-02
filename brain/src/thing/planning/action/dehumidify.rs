use std::fmt::Display;

use anyhow::Result;
use api::command::{Command, PowerToggle, SetPower};

use super::Action;
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Dehumidify;

impl Action for Dehumidify {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        RiskOfMould::Bathroom.current().await
    }

    async fn is_running(&self) -> Result<bool> {
        Powered::Dehumidifier.current().await
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
