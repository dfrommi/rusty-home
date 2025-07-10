use std::fmt::Display;

use crate::home::command::{Command, PowerToggle};
use crate::{core::planner::SimpleAction, home::state::ColdAirComingIn};
use anyhow::Result;

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

impl SimpleAction for RequestClosingWindow {
    fn command(&self) -> Command {
        Command::SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: true,
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> Result<bool> {
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
