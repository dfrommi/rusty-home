use r#macro::Id;

use super::{RuleEvaluationContext, SimpleRule};
use crate::automation::RoomWithWindow;
use crate::command::{Command, Fan};
use crate::core::unit::{FanAirflow, FanSpeed};
use crate::t;

use super::Opened;

#[derive(Debug, Clone, Id)]
pub enum SupportVentilationWithFan {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
}

impl SupportVentilationWithFan {
    fn fan(&self) -> Fan {
        match self {
            SupportVentilationWithFan::LivingRoomCeilingFan => Fan::LivingRoomCeilingFan,
            SupportVentilationWithFan::BedroomCeilingFan => Fan::BedroomCeilingFan,
        }
    }
}

impl SimpleRule for SupportVentilationWithFan {
    fn command(&self) -> Command {
        Command::ControlFan {
            device: self.fan(),
            speed: FanAirflow::Reverse(FanSpeed::Medium),
        }
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
        let window = match self {
            SupportVentilationWithFan::LivingRoomCeilingFan => Opened::Room(RoomWithWindow::LivingRoom),
            SupportVentilationWithFan::BedroomCeilingFan => Opened::Room(RoomWithWindow::Bedroom),
        };

        let opened_dp = ctx.current_dp(window)?;
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
