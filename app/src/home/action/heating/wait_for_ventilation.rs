use crate::core::HomeApi;
use std::fmt::Display;

use crate::core::time::DailyTimeRange;
use crate::core::unit::DegreeCelsius;
use crate::home::command::Command;
use crate::t;
use anyhow::{Ok, Result};

use crate::{
    core::planner::SimpleAction,
    home::{action::HeatingZone, state::Opened},
    port::DataPointAccess,
};

use super::trigger_once_and_keep_running;

#[derive(Debug, Clone)]
pub enum DeferHeatingUntilVentilationDone {
    LivingRoom,
    Bedroom,
    Kitchen,
}

impl DeferHeatingUntilVentilationDone {
    fn heating_zone(&self) -> HeatingZone {
        match self {
            DeferHeatingUntilVentilationDone::LivingRoom => HeatingZone::LivingRoom,
            DeferHeatingUntilVentilationDone::Bedroom => HeatingZone::Bedroom,
            DeferHeatingUntilVentilationDone::Kitchen => HeatingZone::Kitchen,
        }
    }

    fn target_temperature(&self) -> DegreeCelsius {
        match self {
            DeferHeatingUntilVentilationDone::LivingRoom => DegreeCelsius(18.5),
            DeferHeatingUntilVentilationDone::Bedroom => DegreeCelsius(18.0),
            DeferHeatingUntilVentilationDone::Kitchen => DegreeCelsius(15.0),
        }
    }

    fn time_range(&self) -> DailyTimeRange {
        t!(6:12 - 12:30)
    }

    fn window(&self) -> Opened {
        match self {
            DeferHeatingUntilVentilationDone::LivingRoom => Opened::LivingRoomWindowOrDoor,
            DeferHeatingUntilVentilationDone::Bedroom => Opened::BedroomWindow,
            DeferHeatingUntilVentilationDone::Kitchen => Opened::KitchenWindow,
        }
    }
}

impl Display for DeferHeatingUntilVentilationDone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeferHeatingUntilVentilationDone[{}]",
            match self {
                DeferHeatingUntilVentilationDone::LivingRoom => "LivingRoom",
                DeferHeatingUntilVentilationDone::Bedroom => "Bedroom",
                DeferHeatingUntilVentilationDone::Kitchen => "Kitchen",
            }
        )
    }
}

impl SimpleAction for DeferHeatingUntilVentilationDone {
    fn command(&self) -> Command {
        Command::SetHeating {
            device: self.heating_zone().thermostat(),
            target_state: crate::home::command::HeatingTargetState::Heat {
                temperature: self.target_temperature(),
                duration: self.time_range().duration(),
            },
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool> {
        let time_range = match self.time_range().active() {
            Some(range) => range,
            None => return Ok(false),
        };

        let window_opened = self.window().current_data_point(api).await?;

        if time_range.contains(window_opened.timestamp) {
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), *time_range.start(), api).await
    }
}
