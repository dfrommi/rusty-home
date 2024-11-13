use std::fmt::Display;

use crate::thing::state::Powered;
use anyhow::Result;
use api::command::{Command, PowerToggle, SetPower};

use super::{Action, ColdAirComingIn, DataPointAccess};

#[derive(Debug, Clone)]
pub struct RequestClosingWindow;

impl<T> Action<T> for RequestClosingWindow
where
    T: DataPointAccess<ColdAirComingIn> + DataPointAccess<Powered>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        let result: Result<Vec<bool>> = futures::future::join_all([
            api.current(ColdAirComingIn::Bedroom),
            api.current(ColdAirComingIn::Kitchen),
            api.current(ColdAirComingIn::RoomOfRequirements),
        ])
        .await
        .into_iter()
        .collect();

        Ok(result?.into_iter().any(|v| v))
    }

    async fn is_running(&self, api: &T) -> Result<bool> {
        api.current(Powered::LivingRoomNotificationLight).await
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
