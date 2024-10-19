use std::fmt::Display;

use anyhow::{bail, Result};
use api::command::{Command, CommandExecution, CommandTarget, PowerToggle};

use crate::{
    home_api,
    thing::{ColdAirComingIn, DataPointAccess, Executable},
};

use super::{Action, Resource};

#[derive(Debug, Clone)]
pub struct RequestClosingWindow {}

impl Action for RequestClosingWindow {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        let result: Result<Vec<bool>> = futures::future::join_all([
            ColdAirComingIn::Bedroom.current(),
            ColdAirComingIn::Kitchen.current(),
            ColdAirComingIn::RoomOfRequirements.current(),
        ])
        .await
        .into_iter()
        .collect();

        Ok(result?.into_iter().any(|v| v))
    }

    async fn is_running(&self) -> Result<bool> {
        let target = CommandTarget::SetPower(PowerToggle::LivingRoomNotificationLight);

        //TODO hide behind Command interface
        let maybe_command = home_api()
            .get_latest_command(&target)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(
                    "Failed to get latest command, falling back to not running: {}",
                    e
                );
                None
            });

        tracing::debug!("is_running: {:?}", maybe_command);

        match maybe_command {
            Some(CommandExecution {
                command: Command::SetPower { power_on, .. },
                ..
            }) => Ok(power_on),
            Some(cmd) => {
                bail!(
                    "Unexpected command type received for target {:?}: {:?}",
                    target,
                    cmd
                );
            }
            None => Ok(false),
        }
    }

    async fn is_user_controlled(&self) -> Result<bool> {
        Ok(false)
    }

    async fn start(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: true,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: false,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(Resource::LivingRoomNotificationLight)
    }
}

impl Display for RequestClosingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestClosingWindow")
    }
}
