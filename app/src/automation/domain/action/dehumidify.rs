use super::{RuleEvaluationContext, SimpleRule};
use crate::{
    automation::{Room, RoomWithWindow},
    command::{Command, Fan, PowerToggle},
    core::unit::{DegreeCelsius, FanAirflow::Forward, FanSpeed::High, FanSpeed::Medium},
    home_state::{DewPoint, FanActivity, Ventilation},
    t,
};
use anyhow::Result;
use r#macro::{EnumVariants, Id};

use crate::home_state::RiskOfMould;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum Dehumidify {
    Bathroom,
    Bedroom,
}

impl SimpleRule for Dehumidify {
    fn command(&self) -> Command {
        match self {
            Dehumidify::Bathroom => Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            Dehumidify::Bedroom => {
                let speed = if t!(20:00 - 11:00).is_now() {
                    Forward(Medium)
                } else {
                    Forward(High)
                };

                Command::ControlFan {
                    device: Fan::BedroomDehumidifier,
                    speed,
                }
            }
        }
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool> {
        match self {
            Dehumidify::Bathroom => ctx.current(RiskOfMould::Bathroom),
            Dehumidify::Bedroom => {
                let current_fan_state = ctx.current_dp(FanActivity::BedroomDehumidifier)?;
                let current_dewpoint = ctx.current(DewPoint::Room(Room::Bedroom))?;
                let last_ventilation = ctx.current_dp(Ventilation::Room(RoomWithWindow::Bedroom))?.timestamp;

                let running_long_enough =
                    current_fan_state.value.is_on() && current_fan_state.timestamp.elapsed() > t!(30 minutes);
                let was_running_recently =
                    current_fan_state.value.is_off() && current_fan_state.timestamp.elapsed() < t!(60 minutes);
                let ventilated_recently = last_ventilation.elapsed() < t!(45 minutes);

                Ok(!running_long_enough
                    && !was_running_recently
                    && !ventilated_recently
                    && hysterisis_above(
                        current_fan_state.value.is_on(),
                        current_dewpoint,
                        (DegreeCelsius(10.0), DegreeCelsius(10.5)),
                    ))
            }
        }
    }
}

fn hysterisis_above<T>(is_active: bool, current: T, range: (T, T)) -> bool
where
    T: PartialOrd,
{
    let (low, high) = if range.0 < range.1 {
        (range.0, range.1)
    } else {
        (range.1, range.0)
    };

    current > high || (is_active && current >= low && current <= high)
}
