use api::command::{Command, CommandExecution, CommandTarget, PowerToggle};
use goap::{Effects, Preconditions};

use crate::{
    home_api,
    planning::{BedroomState, HomeState, KitchenState, LivingRoomState, RoomOfRequirementsState},
    thing::Executable,
};

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
        let r = !state.bedroom.heating_output_remains
            || !state.kitchen.heating_output_remains
            || !state.room_of_requirements.heating_output_remains;
        tracing::debug!("request closing window: {:?} -- {:?}", r, state);
        r
    }
}

impl Effects<HomeState> for RequestClosingWindow {
    fn apply_to(&self, state: &HomeState) -> HomeState {
        HomeState {
            living_room: LivingRoomState {
                heating_output_remains: true,
                ..state.living_room.clone()
            },
            bedroom: BedroomState {
                heating_output_remains: true,
                ..state.bedroom.clone()
            },
            kitchen: KitchenState {
                heating_output_remains: true,
                ..state.kitchen.clone()
            },
            room_of_requirements: RoomOfRequirementsState {
                heating_output_remains: true,
                ..state.room_of_requirements.clone()
            },

            ..state.clone()
        }
    }
}
