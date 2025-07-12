use crate::core::HomeApi;
use std::fmt::Display;

use crate::core::unit::DegreeCelsius;
use crate::home::command::Command;
use crate::t;
use anyhow::Result;

use crate::{
    core::planner::SimpleAction,
    home::{
        action::HeatingZone,
        state::{AutomaticTemperatureIncrease, OpenedArea},
    },
    port::DataPointAccess,
};

use super::trigger_once_and_keep_running;

static NO_HEATING_SET_POINT: DegreeCelsius = DegreeCelsius(7.0);

#[derive(Debug, Clone)]
pub struct NoHeatingDuringAutomaticTemperatureIncrease {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringAutomaticTemperatureIncrease {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self {
            heating_zone: heating_zone.clone(),
        }
    }
}

impl Display for NoHeatingDuringAutomaticTemperatureIncrease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoHeatingDuringAutomaticTemperatureIncrease[{}]", self.heating_zone)
    }
}

impl SimpleAction for NoHeatingDuringAutomaticTemperatureIncrease {
    fn command(&self) -> Command {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: crate::home::command::HeatingTargetState::Heat {
                temperature: NO_HEATING_SET_POINT,
                duration: t!(1 hours),
            },
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool> {
        let (temp_increase, window_opened) = match self.heating_zone {
            HeatingZone::LivingRoom => (AutomaticTemperatureIncrease::LivingRoom, OpenedArea::LivingRoomWindowOrDoor),
            HeatingZone::Bedroom => (AutomaticTemperatureIncrease::Bedroom, OpenedArea::BedroomWindow),
            HeatingZone::Kitchen => (AutomaticTemperatureIncrease::Kitchen, OpenedArea::KitchenWindow),
            HeatingZone::RoomOfRequirements => (
                AutomaticTemperatureIncrease::RoomOfRequirements,
                OpenedArea::RoomOfRequirementsWindow,
            ),
            HeatingZone::Bathroom => (AutomaticTemperatureIncrease::Bedroom, OpenedArea::BedroomWindow),
        };

        let (window_opened, temp_increase) =
            tokio::try_join!(window_opened.current_data_point(api), temp_increase.current(api))?;

        //window still open or no temp increase
        if !temp_increase || window_opened.value {
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), window_opened.timestamp, api).await
    }
}
