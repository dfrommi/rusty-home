use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::{Command, SetHeating};
use support::{t, time::DailyTimeRange, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{Action, HeatingZone},
    DataPointAccess, Opened,
};

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
            heating_zone,
            target_temperature,
            time_range,
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

impl Action for DeferHeatingUntilVentilationDone {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        if !self.time_range.contains(t!(now)) {
            return Ok(false);
        }

        let window_opened = self.window().current_data_point().await?;
        if self.time_range.contains(window_opened.timestamp) {
            return Ok(false);
        }

        let (already_triggered, has_expected_manual_heating) = tokio::try_join!(
            self.heating_zone.manual_heating_already_triggrered(
                self.target_temperature,
                self.time_range.prev_start(),
            ),
            self.heating_zone
                .is_manual_heating_to(self.target_temperature)
        )?;

        Ok(!already_triggered.value || has_expected_manual_heating.value)
    }

    async fn is_running(&self) -> Result<bool> {
        let has_expected_manual_heating = self
            .heating_zone
            .is_manual_heating_to(self.target_temperature)
            .await?;

        Ok(has_expected_manual_heating.value
            && self
                .time_range
                .contains(has_expected_manual_heating.timestamp))
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Heat {
                    temperature: self.target_temperature,
                    until: self.time_range.for_today().1,
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

impl Display for DeferHeatingUntilVentilationDone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeferHeatingUntilVentilationDone[{} -> {} ({})]",
            self.heating_zone, self.target_temperature, self.time_range
        )
    }
}
