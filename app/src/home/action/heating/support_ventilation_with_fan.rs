use std::fmt::Display;

use crate::core::HomeApi;
use crate::home::command::{Command, Fan};
use crate::home::state::{FanAirflow, FanSpeed};
use crate::t;

use crate::core::planner::SimpleAction;

use super::{DataPointAccess, OpenedArea};

#[derive(Debug, Clone)]
pub struct SupportVentilationWithFan {
    fan: Fan,
}

impl SupportVentilationWithFan {
    pub fn new(fan: Fan) -> Self {
        Self { fan }
    }
}

impl Display for SupportVentilationWithFan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SupportVentilationWithFan[{:?}]", self.fan)
    }
}

impl SimpleAction for SupportVentilationWithFan {
    fn command(&self) -> Command {
        Command::ControlFan {
            device: self.fan.clone(),
            speed: FanAirflow::Forward(FanSpeed::Low),
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let window = match self.fan {
            Fan::LivingRoomCeilingFan => OpenedArea::LivingRoomWindowOrDoor,
            Fan::BedroomCeilingFan => OpenedArea::BedroomWindow,
        };

        let opened_dp = window.current_data_point(api).await?;
        let elapsed = opened_dp.timestamp.elapsed();

        if !opened_dp.value {
            return Ok(false);
        }

        if elapsed < t!(1 minutes) {
            tracing::trace!("Window is open, but for less than a minute");
            return Ok(false);
        } else if elapsed > t!(10 minutes) {
            tracing::trace!("Window is open, but for more than 10 minutes. Stopping active support");
            return Ok(false);
        }

        Ok(true)
    }
}
