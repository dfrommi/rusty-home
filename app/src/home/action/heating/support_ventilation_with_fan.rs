use r#macro::Id;

use crate::core::HomeApi;
use crate::core::unit::DegreeCelsius;
use crate::home::action::SimpleRule;
use crate::home::command::{Command, Fan};
use crate::home::state::{FanAirflow, FanSpeed, Temperature};
use crate::t;

use super::{DataPointAccess, OpenedArea};

#[derive(Debug, Clone, Id)]
pub struct SupportVentilationWithFan(Fan);

impl SupportVentilationWithFan {
    pub fn new(fan: Fan) -> Self {
        Self(fan)
    }
}

impl SimpleRule for SupportVentilationWithFan {
    fn command(&self) -> Command {
        Command::ControlFan {
            device: self.0.clone(),
            speed: FanAirflow::Forward(FanSpeed::Low),
        }
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let (window, temp_sensor) = match self.0 {
            Fan::LivingRoomCeilingFan => (OpenedArea::LivingRoomWindowOrDoor, Temperature::LivingRoomDoor),
            Fan::BedroomCeilingFan => (OpenedArea::BedroomWindow, Temperature::BedroomDoor),
        };

        let opened_dp = window.current_data_point(api).await?;
        let elapsed = opened_dp.timestamp.elapsed();

        if temp_sensor.current(api).await? < DegreeCelsius(24.0) {
            tracing::trace!("Temperature too low, not starting fan");
            return Ok(false);
        }

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
