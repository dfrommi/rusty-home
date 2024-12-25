use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{Command, SetEnergySaving},
    state::Powered,
};
use support::time::DateTime;

use crate::core::{
    planner::{CommandAction, ConditionalAction},
    service::CommandState,
};

use super::{CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub struct SaveTvEnergy;

impl SaveTvEnergy {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for SaveTvEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveTvEnergy")
    }
}

impl CommandAction for SaveTvEnergy {
    fn command(&self) -> Command {
        Command::SetEnergySaving(SetEnergySaving {
            device: api::command::EnergySavingDevice::LivingRoomTv,
            on: true,
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<API> ConditionalAction<API> for SaveTvEnergy
where
    API: DataPointAccess<Powered> + CommandAccess<Command> + CommandState<Command>,
{
    async fn preconditions_fulfilled(&self, api: &API) -> Result<bool> {
        let is_tv_on = api
            .current_data_point(api::state::Powered::LivingRoomTv)
            .await?;

        Ok(is_tv_on.value
            && preconditions_for_oneshot_fulfilled(self, is_tv_on.timestamp, api).await?)
    }
}

async fn preconditions_for_oneshot_fulfilled<API>(
    action: &impl CommandAction,
    since: DateTime,
    api: &API,
) -> Result<bool>
where
    API: CommandAccess<Command> + CommandState<Command>,
{
    let this_source = action.source();
    let this_command = action.command();

    let (last_source, is_reflected) = tokio::try_join!(
        api.get_latest_command_source(action.command(), since),
        api.is_reflected_in_state(&this_command),
    )?;

    Ok(last_source != Some(this_source) || is_reflected)
}
