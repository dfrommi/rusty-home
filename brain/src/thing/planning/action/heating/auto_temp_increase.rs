use std::fmt::Display;

use anyhow::Result;
use api::command::{Command, SetHeating};
use support::{t, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{Action, HeatingZone},
    AutomaticTemperatureIncrease, DataPointAccess,
};

#[derive(Debug, Clone)]
pub struct NoHeatingDuringAutomaticTemperatureIncrease {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringAutomaticTemperatureIncrease {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self { heating_zone }
    }
}

impl Action for NoHeatingDuringAutomaticTemperatureIncrease {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        match self.heating_zone {
            HeatingZone::LivingRoom => AutomaticTemperatureIncrease::LivingRoom,
            HeatingZone::Bedroom => AutomaticTemperatureIncrease::Bedroom,
            HeatingZone::Kitchen => AutomaticTemperatureIncrease::Kitchen,
            HeatingZone::RoomOfRequirements => AutomaticTemperatureIncrease::RoomOfRequirements,
            HeatingZone::Bathroom => AutomaticTemperatureIncrease::Bedroom,
        }
        .current()
        .await
    }

    async fn is_running(&self) -> Result<bool> {
        self.heating_zone
            .current_set_point()
            .current()
            .await
            .map(|v| v == DegreeCelsius(7.1))
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Heat {
                    temperature: DegreeCelsius(7.1),
                    until: t!(in 1 hours),
                },
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Auto,
            }
            .into(),
        )
    }
}

impl Display for NoHeatingDuringAutomaticTemperatureIncrease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NoHeatingDuringAutomaticTemperatureIncrease[{}]",
            self.heating_zone
        )
    }
}
