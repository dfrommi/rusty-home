use r#macro::{EnumVariants, Id};

use crate::{
    automation::{HeatingZone, Radiator},
    core::{time::DateTime, unit::DegreeCelsius},
    home_state::HeatingMode,
};

use crate::home_state::{
    TargetHeatingMode,
    calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    Radiator(Radiator),
}

fn from_iso(s: &str) -> DateTime {
    DateTime::from_iso(s).expect("Invalid ISO datetime")
}

pub struct SetPointStateProvider;

impl DerivedStateProvider<SetPoint, DegreeCelsius> for SetPointStateProvider {
    fn calculate_current(&self, id: SetPoint, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        use crate::device_state::SetPoint as DeviceSetPoint;

        match id {
            SetPoint::Radiator(Radiator::RoomOfRequirements) if from_iso("2025-11-22T15:08:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::LivingRoomBig) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::LivingRoomSmall) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::Kitchen) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Kitchen))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::Bedroom) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::Bathroom) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Bathroom))?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Radiator(Radiator::RoomOfRequirements) => {
                ctx.device_state(DeviceSetPoint::RoomOfRequirements).map(|dp| dp.value)
            }
            SetPoint::Radiator(Radiator::LivingRoomBig) => {
                ctx.device_state(DeviceSetPoint::LivingRoomBig).map(|dp| dp.value)
            }
            SetPoint::Radiator(Radiator::LivingRoomSmall) => {
                ctx.device_state(DeviceSetPoint::LivingRoomSmall).map(|dp| dp.value)
            }
            SetPoint::Radiator(Radiator::Bedroom) => ctx.device_state(DeviceSetPoint::Bedroom).map(|dp| dp.value),
            SetPoint::Radiator(Radiator::Kitchen) => ctx.device_state(DeviceSetPoint::Kitchen).map(|dp| dp.value),
            SetPoint::Radiator(Radiator::Bathroom) => ctx.device_state(DeviceSetPoint::Bathroom).map(|dp| dp.value),
        }
    }
}

fn setpoint_for_mode(id: &SetPoint, mode: &HeatingMode) -> DegreeCelsius {
    let t = match (id, mode) {
        (_, HeatingMode::Manual(t, _)) => t.0,
        (_, HeatingMode::Ventilation) => 0.0,
        (SetPoint::Radiator(Radiator::LivingRoomBig), HeatingMode::EnergySaving) => 19.0,
        (SetPoint::Radiator(Radiator::LivingRoomBig), HeatingMode::PostVentilation) => 19.0,
        (SetPoint::Radiator(Radiator::LivingRoomBig), HeatingMode::Sleep) => 18.5,
        (SetPoint::Radiator(Radiator::LivingRoomBig), HeatingMode::Comfort) => 19.5,
        (SetPoint::Radiator(Radiator::LivingRoomBig), HeatingMode::Away) => 17.0,
        (SetPoint::Radiator(Radiator::LivingRoomSmall), HeatingMode::EnergySaving) => 19.0,
        (SetPoint::Radiator(Radiator::LivingRoomSmall), HeatingMode::PostVentilation) => 19.0,
        (SetPoint::Radiator(Radiator::LivingRoomSmall), HeatingMode::Sleep) => 18.5,
        (SetPoint::Radiator(Radiator::LivingRoomSmall), HeatingMode::Comfort) => 19.5,
        (SetPoint::Radiator(Radiator::LivingRoomSmall), HeatingMode::Away) => 17.0,
        (SetPoint::Radiator(Radiator::RoomOfRequirements), HeatingMode::EnergySaving) => 18.0,
        (SetPoint::Radiator(Radiator::RoomOfRequirements), HeatingMode::PostVentilation) => 18.0,
        (SetPoint::Radiator(Radiator::RoomOfRequirements), HeatingMode::Sleep) => 17.0,
        (SetPoint::Radiator(Radiator::RoomOfRequirements), HeatingMode::Comfort) => 19.0,
        (SetPoint::Radiator(Radiator::RoomOfRequirements), HeatingMode::Away) => 16.0,
        (SetPoint::Radiator(Radiator::Bedroom), HeatingMode::EnergySaving) => 17.5,
        (SetPoint::Radiator(Radiator::Bedroom), HeatingMode::PostVentilation) => 17.0,
        (SetPoint::Radiator(Radiator::Bedroom), HeatingMode::Sleep) => 18.5,
        (SetPoint::Radiator(Radiator::Bedroom), HeatingMode::Comfort) => 19.0,
        (SetPoint::Radiator(Radiator::Bedroom), HeatingMode::Away) => 16.5,
        (SetPoint::Radiator(Radiator::Kitchen), HeatingMode::EnergySaving) => 17.0,
        (SetPoint::Radiator(Radiator::Kitchen), HeatingMode::PostVentilation) => 16.5,
        (SetPoint::Radiator(Radiator::Kitchen), HeatingMode::Sleep) => 16.5,
        (SetPoint::Radiator(Radiator::Kitchen), HeatingMode::Comfort) => 18.0,
        (SetPoint::Radiator(Radiator::Kitchen), HeatingMode::Away) => 16.0,
        (SetPoint::Radiator(Radiator::Bathroom), _) => 15.0,
    };

    DegreeCelsius(t)
}
