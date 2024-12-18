use std::fmt::Display;

use anyhow::{Ok, Result};
use api::{
    command::{SetHeating, Thermostat},
    state::{ExternalAutoControl, SetPoint},
};
use support::{time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    planning::{
        action::{Action, HeatingZone},
        planner::ActionExecution,
    },
    port::{CommandAccess, DataPointAccess},
    state::Resident,
};

#[derive(Debug, Clone)]
pub struct ExtendHeatingUntilSleeping {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
    execution: ActionExecution,
}

impl ExtendHeatingUntilSleeping {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        time_range: DailyTimeRange,
    ) -> Self {
        let action_name = format!(
            "ExtendHeatingUntilSleeping[{} -> {} ({})]",
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
        let time_range = match self.time_range.active() {
            Some(range) => range,
            None => return Ok(false),
        };

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
