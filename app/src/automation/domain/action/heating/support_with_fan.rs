use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::automation::{HeatingZone, Radiator, Room, RoomWithWindow};
use crate::command::{Command, Fan};
use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, FanAirflow, FanSpeed};
use crate::home_state::{FanActivity, HeatingMode, TargetHeatingMode, Temperature};
use crate::t;

use super::Opened;

#[derive(Debug, Clone, Id)]
pub enum SupportWithFan {
    LivingRoomVentilation,
    BedroomVentilation,
    BedroomDehumidification,
    BedroomHeating,
    LivingRoomHeating,
}

impl Rule for SupportWithFan {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let command = match self {
            SupportWithFan::LivingRoomVentilation => ventilation(
                Fan::LivingRoomCeilingFan,
                ctx.current_dp(Opened::Room(RoomWithWindow::LivingRoom))?,
            ),
            SupportWithFan::BedroomVentilation => {
                ventilation(Fan::BedroomCeilingFan, ctx.current_dp(Opened::Room(RoomWithWindow::Bedroom))?)
            }
            SupportWithFan::BedroomDehumidification => {
                dehumidify(Fan::BedroomCeilingFan, ctx.current(FanActivity::BedroomDehumidifier)?.is_on())
            }
            SupportWithFan::BedroomHeating => heating(
                Fan::BedroomCeilingFan,
                FanSpeed::Low,
                ctx.current(Temperature::Room(Room::Bedroom))?,
                ctx.current(Temperature::Radiator(Radiator::Bedroom))?,
                ctx.current(Temperature::RadiatorIn15Minutes(Radiator::Bedroom))?,
            ),
            SupportWithFan::LivingRoomHeating => {
                let room_temp_now = ctx.current(Temperature::Room(Room::Bedroom))?;
                let speed = match ctx.current(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))? {
                    HeatingMode::Comfort => FanSpeed::Silent,
                    _ => FanSpeed::Low,
                };

                let small = heating(
                    Fan::LivingRoomCeilingFan,
                    speed.clone(),
                    room_temp_now,
                    ctx.current(Temperature::Radiator(Radiator::LivingRoomSmall))?,
                    ctx.current(Temperature::RadiatorIn15Minutes(Radiator::LivingRoomSmall))?,
                );

                let big = heating(
                    Fan::LivingRoomCeilingFan,
                    speed,
                    room_temp_now,
                    ctx.current(Temperature::Radiator(Radiator::LivingRoomBig))?,
                    ctx.current(Temperature::RadiatorIn15Minutes(Radiator::LivingRoomBig))?,
                );

                big.or(small)
            }
        };

        Ok(match command {
            Some(command) => RuleResult::Execute(vec![command]),
            None => RuleResult::Skip,
        })
    }
}

fn heating(
    device: Fan,
    speed: FanSpeed,
    room_temp_now: DegreeCelsius,
    radiator_temp_now: DegreeCelsius,
    radiator_temp_in_15min: DegreeCelsius,
) -> Option<Command> {
    let diff_now = radiator_temp_now - room_temp_now;
    let diff_in_15min = radiator_temp_in_15min - room_temp_now;

    if (diff_now > DegreeCelsius(5.0) && diff_in_15min > DegreeCelsius(10.5)) || diff_now > DegreeCelsius(10.0) {
        Some(Command::ControlFan {
            device,
            speed: FanAirflow::Reverse(speed),
        })
    } else {
        None
    }
}

fn ventilation(device: Fan, window: DataPoint<bool>) -> Option<Command> {
    let elapsed = window.timestamp.elapsed();

    if !window.value {
        return None;
    }

    if elapsed < t!(1 minutes) {
        tracing::trace!("Window is open, but for less than a minute");
        return None;
    } else if elapsed > t!(10 minutes) {
        tracing::trace!("Window is open, but for more than 10 minutes. Stopping active support");
        return None;
    }

    Some(Command::ControlFan {
        device,
        speed: FanAirflow::Reverse(FanSpeed::Medium),
    })
}

fn dehumidify(device: Fan, is_on: bool) -> Option<Command> {
    if !is_on {
        return None;
    }

    Some(Command::ControlFan {
        device,
        speed: FanAirflow::Reverse(FanSpeed::Medium),
    })
}
