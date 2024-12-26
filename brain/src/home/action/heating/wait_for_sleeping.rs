use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::{Command, SetHeating};
use support::{time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    home::{action::HeatingZone, state::Resident},
    port::{CommandAccess, DataPointAccess},
};

use super::{trigger_once_and_keep_running, CommandState};

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
            heating_zone: heating_zone.clone(),
            target_temperature,
            time_range: time_range.clone(),
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

impl CommandAction for ExtendHeatingUntilSleeping {
    fn command(&self) -> Command {
        Command::SetHeating(SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature,
                duration: self.time_range.duration(),
            },
        })
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for ExtendHeatingUntilSleeping
where
    T: DataPointAccess<Resident> + CommandAccess<Command> + CommandState<Command>,
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

        trigger_once_and_keep_running(&self.command(), &self.source(), *time_range.start(), api)
            .await
    }
}
