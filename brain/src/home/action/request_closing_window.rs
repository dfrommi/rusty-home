use std::fmt::Display;

use crate::home::state::{ColdAirComingIn, Powered};
use anyhow::Result;
use api::command::{PowerToggle, SetPower};

use super::{Action, ActionExecution, DataPointAccess};

#[derive(Debug, Clone)]
pub struct RequestClosingWindow;

impl RequestClosingWindow {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T> Action<T, SetPower> for RequestClosingWindow
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

    fn execution(&self) -> ActionExecution<SetPower> {
        ActionExecution::start_stop(
            self.to_string(),
            SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: false,
            },
        )
    }
}

impl Display for RequestClosingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestClosingWindow")
    }
}
