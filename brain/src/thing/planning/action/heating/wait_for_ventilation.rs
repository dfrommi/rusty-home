use std::fmt::Display;

use anyhow::{Ok, Result};
use api::{
    command::{Command, SetHeating, Thermostat},
    state::{ExternalAutoControl, SetPoint},
};
use support::{t, time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    port::{CommandAccess, DataPointAccess},
    thing::{
        planning::action::{Action, HeatingZone},
        state::Opened,
    },
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

impl<T> Action<T> for DeferHeatingUntilVentilationDone
where
    T: DataPointAccess<Opened>
        + DataPointAccess<SetPoint>
        + DataPointAccess<ExternalAutoControl>
        + CommandAccess<Thermostat>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        let time_range = self.time_range.starting_today();
        if !time_range.contains(t!(now)) {
            return Ok(false);
        }

        let window_opened = api.current_data_point(self.window()).await?;

        if time_range.contains(window_opened.timestamp) {
            return Ok(false);
        }

        let (already_triggered, has_expected_manual_heating) = tokio::try_join!(
            self.heating_zone.manual_heating_already_triggrered(
                api,
                self.target_temperature,
                time_range.start(),
            ),
            self.heating_zone
                .is_manual_heating_to(api, self.target_temperature)
        )?;

        Ok(!already_triggered.value || has_expected_manual_heating.value)
    }

    async fn is_running(&self, api: &T) -> Result<bool> {
        let has_expected_manual_heating = self
            .heating_zone
            .is_manual_heating_to(api, self.target_temperature)
            .await?;

        Ok(has_expected_manual_heating.value
            && self
                .time_range
                .starting_today()
                .contains(has_expected_manual_heating.timestamp))
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Heat {
                    temperature: self.target_temperature,
                    until: self.time_range.starting_today().end(),
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
