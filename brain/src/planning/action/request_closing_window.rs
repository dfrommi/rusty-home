use api::command::{Command, CommandExecution, CommandTarget, PowerToggle};
use goap::{Effects, Preconditions};

use crate::{home_api, planning::HomeState, thing::Executable};

use super::Action;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestClosingWindow {}

impl Action for RequestClosingWindow {
    async fn start(&self) -> anyhow::Result<()> {
        if self.is_running().await {
            tracing::debug!("Already running");
            return Ok(());
        }

        Command::SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: true,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Command::SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on: false,
        }
        .execute()
        .await
    }

    async fn is_running(&self) -> bool {
        let target = CommandTarget::SetPower(PowerToggle::LivingRoomNotificationLight);

        //TODO hide behind Command interface
        let maybe_command = home_api()
            .get_latest_command(&target)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(
                    "Failed to get latest command, falling back to not running: {}",
                    e
                );
                None
            });

        tracing::debug!("is_running: {:?}", maybe_command);

        match maybe_command {
            Some(CommandExecution {
                command: Command::SetPower { power_on, .. },
                ..
            }) => power_on,
            Some(cmd) => {
                tracing::error!(
                    "Unexpected command type received for target {:?}: {:?}",
                    target,
                    cmd
                );
                false
            }
            None => false,
        }
    }

    async fn is_enabled(&self) -> bool {
        true //no user override possible
    }
}

impl Preconditions<HomeState> for RequestClosingWindow {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        let r = !state.heating_output_remains_in_bedroom
            || !state.heating_output_remains_in_kitchen
            || !state.heating_output_remains_in_room_of_requirements;
        tracing::debug!("request closing window: {:?} -- {:?}", r, state);
        r
    }
}

impl Effects<HomeState> for RequestClosingWindow {
    fn apply_to(&self, state: &HomeState) -> HomeState {
        HomeState {
            heating_output_remains_in_living_room: true,
            heating_output_remains_in_bedroom: true,
            heating_output_remains_in_kitchen: true,
            heating_output_remains_in_room_of_requirements: true,

            ..state.clone()
        }
    }
}
