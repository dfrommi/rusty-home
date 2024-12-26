use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::{Command, SetHeating};
use support::{time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    home::{action::HeatingZone, state::Opened},
    port::{CommandAccess, DataPointAccess},
};

use super::{trigger_once_and_keep_running, CommandState};

#[derive(Debug, Clone)]
pub struct DeferHeatingUntilVentilationDone {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
}

impl DeferHeatingUntilVentilationDone {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        time_range: DailyTimeRange,
    ) -> Self {
        Self {
            heating_zone: heating_zone.clone(),
            target_temperature,
            time_range: time_range.clone(),
        }
    }

    fn window(&self) -> Opened {
        match self.heating_zone {
            HeatingZone::LivingRoom => Opened::LivingRoomWindowOrDoor,
            HeatingZone::Bedroom => Opened::BedroomWindow,
            HeatingZone::Kitchen => Opened::KitchenWindow,
            HeatingZone::RoomOfRequirements => Opened::LivingRoomWindowOrDoor,
            HeatingZone::Bathroom => Opened::BedroomWindow,
        }
    }
}

impl Display for DeferHeatingUntilVentilationDone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeferHeatingUntilVentilationDone[{} -> {} ({})]",
            self.heating_zone, self.target_temperature, self.time_range
        )
    }
}

impl CommandAction for DeferHeatingUntilVentilationDone {
    fn command(&self) -> Command {
        Command::SetHeating(SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature,
                duration: self.time_range.duration(),
            },
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for DeferHeatingUntilVentilationDone
where
    T: DataPointAccess<Opened> + CommandAccess<Command> + CommandState<Command>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        let time_range = match self.time_range.active() {
            Some(range) => range,
            None => return Ok(false),
        };

        let window_opened = api.current_data_point(self.window()).await?;

        if time_range.contains(window_opened.timestamp) {
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), *time_range.start(), api)
            .await
    }
}
