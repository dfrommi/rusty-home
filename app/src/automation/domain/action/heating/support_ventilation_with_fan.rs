use r#macro::Id;

use crate::command::{Command, Fan};
use crate::core::unit::{DegreeCelsius, FanAirflow, FanSpeed};
use crate::home_state::Temperature;
use crate::t;
use super::{RuleEvaluationContext, SimpleRule};

use super::OpenedArea;

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

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
        let (window, temp_sensor) = match self.0 {
            Fan::LivingRoomCeilingFan => (OpenedArea::LivingRoomWindowOrDoor, Temperature::LivingRoom),
            Fan::BedroomCeilingFan => (OpenedArea::BedroomWindow, Temperature::Bedroom),
        };

        let opened_dp = ctx.current_dp(window)?;
        let elapsed = opened_dp.timestamp.elapsed();

        if ctx.current(temp_sensor)? < DegreeCelsius(24.0) {
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
