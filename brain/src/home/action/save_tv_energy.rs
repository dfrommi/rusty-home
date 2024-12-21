use anyhow::Result;
use api::{
    command::{Command, SetEnergySaving},
    state::Powered,
};
use support::time::DateTime;

use crate::core::planner::{ActionExecutionTrigger, CommandState};

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

        Ok(is_tv_on.value
            && preconditions_for_oneshot_fulfilled(self, is_tv_on.timestamp, api).await?)
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

async fn preconditions_for_oneshot_fulfilled<API, C>(
    action: &impl Action<API, C>,
    since: DateTime,
    api: &API,
) -> Result<bool>
where
    C: Into<Command>,
    API: CommandAccess<C> + CommandState<C>,
{
    let execution = action.execution();

    let (last_trigger, is_still_running) = tokio::try_join!(
        execution.last_trigger_since(api, since),
        execution.is_reflected_in_state(api),
    )?;

    Ok(last_trigger != ActionExecutionTrigger::Start || is_still_running.unwrap_or(false))
}

impl std::fmt::Display for SaveTvEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveTvEnergy")
    }
}
