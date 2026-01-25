use super::{RuleEvaluationContext, SimpleRule};
use crate::{
    automation::{HeatingZone, Room, RoomWithWindow},
    command::{Command, Fan, PowerToggle},
    core::unit::{
        DegreeCelsius,
        FanAirflow::Forward,
        FanSpeed::{High, Medium},
    },
    home_state::{DewPoint, FanActivity, HeatingMode, TargetHeatingMode, Ventilation},
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
            Dehumidify::Bathroom => {
                let risk = ctx.current(RiskOfMould::Bathroom)?;
                if risk {
                    tracing::info!("Risk of mould detected; dehumidifying bathroom");
                } else {
                    tracing::info!("No mould risk; skipping bathroom dehumidification");
                }
                Ok(risk)
            }
            Dehumidify::Bedroom => {
                let current_fan_state = ctx.current_dp(FanActivity::BedroomDehumidifier)?;
                let current_dewpoint = ctx.current(DewPoint::Room(Room::Bedroom))?;
                let last_ventilation = ctx.current_dp(Ventilation::Room(RoomWithWindow::Bedroom))?.timestamp;
                let sleep_mode =
                    ctx.current(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))? == HeatingMode::Sleep;

                //TODO move to central blocker
                if sleep_mode {
                    tracing::info!("Sleep mode active; skipping bedroom dehumidification");
                    return Ok(false);
                }

                if current_fan_state.value.is_on() && current_fan_state.timestamp.elapsed() > t!(30 minutes) {
                    tracing::info!("Dehumidifier fan running for more than 30 minutes; stopping");
                    return Ok(false);
                }

                if current_fan_state.value.is_off() && current_fan_state.timestamp.elapsed() < t!(60 minutes) {
                    tracing::info!("Dehumidifier fan ran within the last 60 minutes; skipping");
                    return Ok(false);
                }

                if last_ventilation.elapsed() < t!(45 minutes) {
                    tracing::info!("Ventilated within the last 45 minutes; skipping dehumidification");
                    return Ok(false);
                }

                let above_dewpoint = hysterisis_above(
                    current_fan_state.value.is_on(),
                    current_dewpoint,
                    (DegreeCelsius(10.0), DegreeCelsius(10.5)),
                );

                if above_dewpoint {
                    tracing::info!("Dewpoint high enough; dehumidifying bedroom");
                    Ok(true)
                } else {
                    tracing::info!("Dewpoint low enough; skipping dehumidification");
                    Ok(false)
                }
            }
        }
    }
}

fn hysterisis_above<T>(is_active: bool, current: T, range: (T, T)) -> bool
where
    T: PartialOrd + std::fmt::Display,
{
    let (low, high) = if range.0 < range.1 {
        (range.0, range.1)
    } else {
        (range.1, range.0)
    };

    if current > high {
        tracing::debug!("Value {current} is above high threshold {high}; enabled");
        return true;
    } else if current < low {
        tracing::debug!("Value {current} is below low threshold {low}; disabled");
        return false;
    }

    //in hysteresis range
    if is_active {
        tracing::debug!(
            "Value {current} is within hysteresis range ({low} - {high}) and currently active; remains enabled"
        );

        true
    } else {
        tracing::debug!(
            "Value {current} is within hysteresis range ({low} - {high}) and currently inactive; remains disabled"
        );

        false
    }
}
