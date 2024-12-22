use std::fmt::Display;

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    home::state::{ColdAirComingIn, Powered},
};
use anyhow::Result;
use api::command::{Command, PowerToggle, SetPower};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub struct RequestClosingWindow;

impl RequestClosingWindow {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for RequestClosingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestClosingWindow")
    }
}

impl CommandAction for RequestClosingWindow {
    fn command(&self) -> Command {
        Command::SetPower(SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: true,
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for RequestClosingWindow
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
}
