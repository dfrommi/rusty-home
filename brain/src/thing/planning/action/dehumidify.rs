use std::fmt::Display;

use anyhow::Result;
use api::command::{Command, PowerToggle};

use crate::thing::Executable;

use super::{Action, Resource};
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

    async fn start(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: false,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(Resource::Dehumidifier)
    }
}

impl Display for Dehumidify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dehumidify")
    }
}
