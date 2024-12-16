use anyhow::Result;
use api::{
    command::{Command, EnergySavingDevice, SetEnergySaving},
    state::Powered,
};

use crate::planning::planner::{ActionExecutionState, CommandState};

use super::{Action, CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub struct SaveTvEnergy;

impl<API> Action<API> for SaveTvEnergy
where
    API: DataPointAccess<Powered> + CommandAccess<EnergySavingDevice> + CommandAccess<Command>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool> {
        let is_tv_on = api
            .current_data_point(api::state::Powered::LivingRoomTv)
            .await?;

        if !is_tv_on.value {
            return Ok(false);
        }

        let command = SetEnergySaving {
            device: api::command::EnergySavingDevice::LivingRoomTv,
            on: true,
        };

        let (was_started, is_still_running) = tokio::try_join!(
            ActionExecutionState::was_started_since(self, api, is_tv_on.timestamp),
            CommandState::is_running(&command, api),
        )?;

        Ok(!was_started || is_still_running)
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetEnergySaving {
                device: api::command::EnergySavingDevice::LivingRoomTv,
                on: true,
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<Command> {
        None
    }
}

impl std::fmt::Display for SaveTvEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveTvEnergy")
    }
}
