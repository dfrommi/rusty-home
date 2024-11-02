use std::fmt::Display;

use crate::thing::state::Powered;
use crate::thing::{ColdAirComingIn, DataPointAccess};
use anyhow::Result;
use api::command::{Command, PowerToggle, SetPower};

use super::Action;

#[derive(Debug, Clone)]
pub struct RequestClosingWindow;

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
        Powered::LivingRoomNotificationLight.current().await
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<Command> {
        Some(
            SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: false,
            }
            .into(),
        )
    }
}

impl Display for RequestClosingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestClosingWindow")
    }
}
