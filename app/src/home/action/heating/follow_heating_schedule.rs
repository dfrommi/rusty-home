use std::fmt::Display;

use crate::{
    core::planner::SimpleAction,
    home::{
        action::HeatingZone,
        command::{Command, HeatingTargetState},
        state::{HeatingMode, ScheduledHeatingMode},
    },
    port::DataPointAccess,
};

#[derive(Debug, Clone)]
pub struct FollowHeatingSchedule {
    zone: HeatingZone,
    mode: HeatingMode,
}

impl FollowHeatingSchedule {
    pub fn new(zone: HeatingZone, mode: HeatingMode) -> Self {
        Self { zone, mode }
    }
}

impl Display for FollowHeatingSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FollowHeatingSchedule[{} - {}]", self.zone, self.mode.int_name())
    }
}

impl SimpleAction for FollowHeatingSchedule {
    fn command(&self) -> Command {
        Command::SetHeating {
            target_state: HeatingTargetState::for_mode(&self.mode, &self.zone.thermostat()),
            device: self.zone.thermostat(),
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> anyhow::Result<bool> {
        let active_mode = match self.zone {
            HeatingZone::RoomOfRequirements => ScheduledHeatingMode::RoomOfRequirements,
            HeatingZone::LivingRoom => ScheduledHeatingMode::LivingRoom,
            HeatingZone::Bedroom => ScheduledHeatingMode::Bedroom,
            HeatingZone::Kitchen => ScheduledHeatingMode::Kitchen,
            HeatingZone::Bathroom => ScheduledHeatingMode::Bathroom,
        };

        Ok(active_mode.current(api).await? == self.mode)
    }
}
