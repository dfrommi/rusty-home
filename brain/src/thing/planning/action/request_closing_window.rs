use std::fmt::Display;

use crate::thing::state::Powered;
use crate::thing::{ColdAirComingIn, DataPointAccess, Executable};
use anyhow::Result;
use api::command::{Command, PowerToggle, SetPower};

use super::{Action, Resource};

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

    async fn start(&self) -> Result<()> {
        SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: true,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        SetPower {
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
