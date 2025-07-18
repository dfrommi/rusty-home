use crate::core::HomeApi;
use std::fmt::Display;

use crate::core::time::DailyTimeRange;
use crate::core::unit::DegreeCelsius;
use crate::home::command::Command;
use crate::t;
use anyhow::{Ok, Result};

use crate::{
    core::planner::SimpleAction,
    home::{action::HeatingZone, state::Resident},
    port::DataPointAccess,
};

use super::trigger_once_and_keep_running;

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
            ExtendHeatingUntilSleeping::LivingRoom => DegreeCelsius(19.5),
            ExtendHeatingUntilSleeping::Bedroom => DegreeCelsius(18.0),
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

impl SimpleAction for ExtendHeatingUntilSleeping {
    fn command(&self) -> Command {
        Command::SetHeating {
            device: self.heating_zone().thermostat(),
            target_state: crate::home::command::HeatingTargetState::Heat {
                temperature: self.target_temperature(),
                duration: self.time_range().duration(),
            },
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    //Strong overlap with wait_for_ventilation
    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool> {
        let time_range = match self.time_range().active() {
            Some(range) => range,
            None => return Ok(false),
        };

        let (dennis, sabine) =
            tokio::try_join!(Resident::DennisSleeping.current(api), Resident::SabineSleeping.current(api),)?;

        if dennis || sabine {
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), *time_range.start(), api).await
    }
}
