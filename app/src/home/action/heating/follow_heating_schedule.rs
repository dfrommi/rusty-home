use std::fmt::Display;

use crate::{
    core::planner::SimpleAction,
    home::{
        command::{Command, HeatingTargetState, Thermostat},
        state::{HeatingMode, ScheduledHeatingMode},
    },
    port::DataPointAccess,
};

#[derive(Debug, Clone)]
pub enum FollowHeatingSchedule {
    RoomOfRequirements(HeatingMode),
}

impl FollowHeatingSchedule {
    fn heating_mode(&self) -> &HeatingMode {
        match self {
            FollowHeatingSchedule::RoomOfRequirements(mode) => mode,
        }
    }
}

impl Display for FollowHeatingSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FollowHeatingSchedule[{} - {}]",
            match self {
                FollowHeatingSchedule::RoomOfRequirements(_) => "RoomOfRequirements",
            },
            self.heating_mode()
        )
    }
}

impl SimpleAction for FollowHeatingSchedule {
    fn command(&self) -> Command {
        let thermostat = match self {
            FollowHeatingSchedule::RoomOfRequirements(_) => Thermostat::RoomOfRequirements,
        };

        let mode = self.heating_mode();

        Command::SetHeating {
            target_state: HeatingTargetState::for_mode(mode, &thermostat),
            device: thermostat,
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> anyhow::Result<bool> {
        let active_mode = match self {
            FollowHeatingSchedule::RoomOfRequirements(_) => ScheduledHeatingMode::RoomOfRequirements,
        };

        Ok(&active_mode.current(api).await? == self.heating_mode())
    }
}
