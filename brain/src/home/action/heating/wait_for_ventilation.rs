use std::fmt::Display;

use anyhow::{Ok, Result};
use api::{
    command::{SetHeating, Thermostat},
    state::{ExternalAutoControl, SetPoint},
};
use support::{time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    home::action::{Action, HeatingZone},
    home::state::Opened,
    port::{CommandAccess, DataPointAccess},
};

use super::ActionExecution;

#[derive(Debug, Clone)]
pub struct DeferHeatingUntilVentilationDone {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
    execution: ActionExecution,
}

impl DeferHeatingUntilVentilationDone {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        time_range: DailyTimeRange,
    ) -> Self {
        let action_name = format!(
            "DeferHeatingUntilVentilationDone[{} -> {} ({})]",
            &heating_zone, &target_temperature, &time_range
        );

        Self {
            heating_zone: heating_zone.clone(),
            target_temperature,
            time_range: time_range.clone(),
            execution: ActionExecution::from_start_and_stop(
                action_name.as_str(),
                SetHeating {
                    device: heating_zone.thermostat(),
                    target_state: api::command::HeatingTargetState::Heat {
                        temperature: target_temperature,
                        until: time_range.next_end(),
                    },
                },
                SetHeating {
                    device: heating_zone.thermostat(),
                    target_state: api::command::HeatingTargetState::Auto,
                },
            ),
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
        let time_range = match self.time_range.active() {
            Some(range) => range,
            None => return Ok(false),
        };

        let window_opened = api.current_data_point(self.window()).await?;

        if time_range.contains(window_opened.timestamp) {
            return Ok(false);
        }

        let (already_triggered, has_expected_manual_heating) = tokio::try_join!(
            self.heating_zone.manual_heating_already_triggrered(
                api,
                self.target_temperature,
                *time_range.start(),
            ),
            self.heating_zone
                .is_manual_heating_to(api, self.target_temperature)
        )?;

        Ok(!already_triggered.value || has_expected_manual_heating.value)
    }

    fn execution(&self) -> &ActionExecution {
        &self.execution
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
