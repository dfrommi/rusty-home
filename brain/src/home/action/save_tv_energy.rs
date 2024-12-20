use anyhow::Result;
use api::{command::SetEnergySaving, state::Powered};

use crate::core::planner::ActionExecutionTrigger;

use super::{Action, ActionExecution, CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub struct SaveTvEnergy;

impl SaveTvEnergy {
    pub fn new() -> Self {
        Self {}
    }
}

impl<API> Action<API, SetEnergySaving> for SaveTvEnergy
where
    API: DataPointAccess<Powered> + CommandAccess<SetEnergySaving>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool> {
        let is_tv_on = api
            .current_data_point(api::state::Powered::LivingRoomTv)
            .await?;

        if !is_tv_on.value {
            return Ok(false);
        }

        let execution = <Self as Action<API, SetEnergySaving>>::execution(self);
        let (was_started, is_still_running) = tokio::try_join!(
            execution.any_trigger_since(api, ActionExecutionTrigger::Start, is_tv_on.timestamp),
            execution.is_reflected_in_state(api),
        )?;

        Ok(!was_started || is_still_running)
    }

    fn execution(&self) -> ActionExecution<SetEnergySaving> {
        ActionExecution::from_start(
            self.to_string(),
            api::command::SetEnergySaving {
                device: api::command::EnergySavingDevice::LivingRoomTv,
                on: true,
            },
        )
    }
}

impl std::fmt::Display for SaveTvEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveTvEnergy")
    }
}
