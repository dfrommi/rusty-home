use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::Command;
use support::{t, time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    core::planner::{CommandAction, ConditionalAction},
    home::{action::HeatingZone, state::Resident},
    port::{CommandAccess, DataPointAccess},
};

use super::{trigger_once_and_keep_running, CommandState};

#[derive(Debug, Clone)]
pub enum ExtendHeatingUntilSleeping {
    LivingRoom,
    Bedroom,
}

impl ExtendHeatingUntilSleeping {
    fn heating_zone(&self) -> HeatingZone {
        match self {
            ExtendHeatingUntilSleeping::LivingRoom => HeatingZone::LivingRoom,
            ExtendHeatingUntilSleeping::Bedroom => HeatingZone::Bedroom,
        }
    }

    fn target_temperature(&self) -> DegreeCelsius {
        match self {
            ExtendHeatingUntilSleeping::LivingRoom => DegreeCelsius(20.0),
            ExtendHeatingUntilSleeping::Bedroom => DegreeCelsius(19.0),
        }
    }

    fn time_range(&self) -> DailyTimeRange {
        t!(22:30 - 2:30)
    }
}

impl Display for ExtendHeatingUntilSleeping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExtendHeatingUntilSleeping[{}]",
            match self {
                ExtendHeatingUntilSleeping::LivingRoom => "LivingRoom",
                ExtendHeatingUntilSleeping::Bedroom => "Bedroom",
            }
        )
    }
}

impl CommandAction for ExtendHeatingUntilSleeping {
    fn command(&self) -> Command {
        Command::SetHeating {
            device: self.heating_zone().thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature(),
                duration: self.time_range().duration(),
            },
        }
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for ExtendHeatingUntilSleeping
where
    T: DataPointAccess<Resident> + CommandAccess + CommandState,
{
    //Strong overlap with wait_for_ventilation
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        let time_range = match self.time_range().active() {
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
