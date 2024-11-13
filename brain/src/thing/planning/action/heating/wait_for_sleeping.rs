use std::fmt::Display;

use anyhow::{Ok, Result};
use api::{
    command::{Command, SetHeating, Thermostat},
    state::{ExternalAutoControl, SetPoint},
};
use support::{t, time::DailyTimeRange, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{Action, HeatingZone},
    state::{DataPointAccess, Resident},
    CommandAccess,
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
        time_range: DailyTimeRange,
    ) -> Self {
        Self {
            heating_zone,
            target_temperature,
            time_range,
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

impl<T> Action<T> for ExtendHeatingUntilSleeping
where
    T: DataPointAccess<Resident>
        + DataPointAccess<SetPoint>
        + DataPointAccess<ExternalAutoControl>
        + CommandAccess<Thermostat>,
{
    //Strong overlap with wait_for_ventilation
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        let time_range = self.time_range.starting_today();

        if !time_range.contains(t!(now)) {
            return Ok(false);
        }

        let (dennis, sabine) = tokio::try_join!(
            api.current(Resident::DennisSleeping),
            api.current(Resident::SabineSleeping),
        )?;

        if dennis || sabine {
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

    //same as wait_for_ventilation, maybe merge
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
