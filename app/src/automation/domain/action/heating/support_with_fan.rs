use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::automation::{Radiator, Room, RoomWithWindow};
use crate::command::{Command, Fan};
use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, FanAirflow, FanSpeed};
use crate::home_state::{FanActivity, Temperature};
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
                let speed = FanSpeed::Silent;

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

    if diff_now > DegreeCelsius(10.0) {
        tracing::info!("Radiator more than 10C warmer than room; supporting heating");
        return Some(Command::ControlFan {
            device,
            speed: FanAirflow::Reverse(speed),
        });
    }

    if diff_now > DegreeCelsius(5.0) && diff_in_15min > DegreeCelsius(10.5) {
        tracing::info!("Radiator more than 5C warmer now and more than 10.5C in 15 minutes; supporting heating");
        return Some(Command::ControlFan {
            device,
            speed: FanAirflow::Reverse(speed),
        });
    }

    tracing::info!("Radiator not warm enough; skipping heating support");
    None
}

fn ventilation(device: Fan, window: DataPoint<bool>) -> Option<Command> {
    let elapsed = window.timestamp.elapsed();

    if !window.value {
        tracing::info!("Window closed; skipping ventilation support");
        return None;
    }

    if elapsed < t!(1 minutes) {
        tracing::info!("Window open for less than 1 minute; skipping ventilation support");
        return None;
    } else if elapsed > t!(10 minutes) {
        tracing::info!("Window open for more than 10 minutes; stopping ventilation support");
        return None;
    }

    tracing::info!("Window open between 1 and 10 minutes; supporting ventilation");
    Some(Command::ControlFan {
        device,
        speed: FanAirflow::Reverse(FanSpeed::Medium),
    })
}

fn dehumidify(device: Fan, is_on: bool) -> Option<Command> {
    if !is_on {
        tracing::info!("Dehumidifier off; skipping dehumidification support");
        return None;
    }

    tracing::info!("Dehumidifier running; supporting dehumidification");
    Some(Command::ControlFan {
        device,
        speed: FanAirflow::Reverse(FanSpeed::Medium),
    })
}
