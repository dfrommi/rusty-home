use std::fmt::Display;

use anyhow::Result;
use api::command::{Command, SetHeating};
use support::{t, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{Action, HeatingZone},
    AutomaticTemperatureIncrease, DataPointAccess, Opened,
};

static NO_HEATING_SET_POINT: DegreeCelsius = DegreeCelsius(7.0);

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
        let (temp_increase, window_opened) = match self.heating_zone {
            HeatingZone::LivingRoom => (
                AutomaticTemperatureIncrease::LivingRoom,
                Opened::LivingRoomWindowOrDoor,
            ),
            HeatingZone::Bedroom => (AutomaticTemperatureIncrease::Bedroom, Opened::BedroomWindow),
            HeatingZone::Kitchen => (AutomaticTemperatureIncrease::Kitchen, Opened::KitchenWindow),
            HeatingZone::RoomOfRequirements => (
                AutomaticTemperatureIncrease::RoomOfRequirements,
                Opened::RoomOfRequirementsWindow,
            ),
            HeatingZone::Bathroom => (AutomaticTemperatureIncrease::Bedroom, Opened::BedroomWindow),
        };

        let (window_opened, temp_increase) =
            tokio::try_join!(window_opened.current_data_point(), temp_increase.current())?;

        //window still open or no temp increase
        if !temp_increase || window_opened.value {
            return Ok(false);
        }

        //another place very similar to the rest
        let (already_triggered, has_expected_manual_heating) = tokio::try_join!(
            self.heating_zone
                .manual_heating_already_triggrered(NO_HEATING_SET_POINT, window_opened.timestamp),
            self.heating_zone.is_manual_heating_to(NO_HEATING_SET_POINT)
        )?;

        Ok(!already_triggered.value || has_expected_manual_heating.value)
    }

    async fn is_running(&self) -> Result<bool> {
        self.heating_zone
            .current_set_point()
            .current()
            .await
            .map(|v| v == NO_HEATING_SET_POINT)
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Heat {
                    temperature: NO_HEATING_SET_POINT,
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
