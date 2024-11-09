use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::{Command, SetHeating};
use support::{t, time::DailyTimeRange, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{Action, HeatingZone},
    DataPointAccess, Resident,
};

#[derive(Debug, Clone)]
pub struct ExtendHeatingUntilSleeping {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
}

impl ExtendHeatingUntilSleeping {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        start_hm: (u32, u32),
        latest_until_hm: (u32, u32),
    ) -> Self {
        Self {
            heating_zone,
            target_temperature,
            time_range: DailyTimeRange::new(start_hm, latest_until_hm),
        }
    }
}

impl Display for ExtendHeatingUntilSleeping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExtendHeatingUntilSleeping[{} -> {} ({})]",
            self.heating_zone, self.target_temperature, self.time_range
        )
    }
}

impl Action for ExtendHeatingUntilSleeping {
    //Strong overlap with wait_for_ventilation
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        if !self.time_range.contains(t!(now)) {
            return Ok(false);
        }

        let (dennis, sabine) = tokio::try_join!(
            Resident::DennisSleeping.current(),
            Resident::SabineSleeping.current(),
        )?;

        if dennis || sabine {
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

    //same as wait_for_ventilation, maybe merge
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
