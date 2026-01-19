use r#macro::Id;

use super::{RuleEvaluationContext, SimpleRule};
use crate::automation::RoomWithWindow;
use crate::command::{Command, Fan};
use crate::core::timeseries::DataPoint;
use crate::core::unit::{FanAirflow, FanSpeed};
use crate::home_state::FanActivity;
use crate::t;

use super::Opened;

#[derive(Debug, Clone, Id)]
pub enum SupportWithFan {
    LivingRoomVentilation,
    BedroomVentilation,
    BedroomDehumidification,
}

impl SimpleRule for SupportWithFan {
    fn command(&self) -> Command {
        let device = match self {
            SupportWithFan::LivingRoomVentilation => Fan::LivingRoomCeilingFan,
            SupportWithFan::BedroomVentilation | SupportWithFan::BedroomDehumidification => Fan::BedroomCeilingFan,
        };

        Command::ControlFan {
            device,
            speed: FanAirflow::Reverse(FanSpeed::Medium),
        }
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
        Ok(match self {
            SupportWithFan::LivingRoomVentilation => {
                ventilation(ctx.current_dp(Opened::Room(RoomWithWindow::LivingRoom))?)
            }
            SupportWithFan::BedroomVentilation => ventilation(ctx.current_dp(Opened::Room(RoomWithWindow::Bedroom))?),
            SupportWithFan::BedroomDehumidification => ctx.current(FanActivity::BedroomDehumidifier)?.is_on(),
        })
    }
}

fn ventilation(window: DataPoint<bool>) -> bool {
    let elapsed = window.timestamp.elapsed();

    if !window.value {
        return false;
    }

    if elapsed < t!(1 minutes) {
        tracing::trace!("Window is open, but for less than a minute");
        return false;
    } else if elapsed > t!(10 minutes) {
        tracing::trace!("Window is open, but for more than 10 minutes. Stopping active support");
        return false;
    }

    true
}
