use anyhow::Result;
use api::{
    command::{Command, EnergySavingDevice, SetEnergySaving},
    state::Powered,
};

use crate::core::planner::{ActionExecutionTrigger, CommandState};

use super::{Action, ActionExecution, CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub struct SaveTvEnergy {
    execution: ActionExecution,
}

impl SaveTvEnergy {
    pub fn new() -> Self {
        Self {
            execution: ActionExecution::from_start(
                "SaveTvEnergy",
                api::command::SetEnergySaving {
                    device: api::command::EnergySavingDevice::LivingRoomTv,
                    on: true,
                },
            ),
        }
    }
}

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
            self.execution.any_trigger_since(
                api,
                ActionExecutionTrigger::Start,
                is_tv_on.timestamp
            ),
            CommandState::is_reflected_in_state(api, &command),
        )?;

        Ok(!was_started || is_still_running)
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}

impl std::fmt::Display for SaveTvEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveTvEnergy")
    }
}
