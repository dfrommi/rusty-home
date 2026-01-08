use r#macro::{EnumVariants, Id};

use crate::{
    core::{time::DateTime, unit::DegreeCelsius},
    home_state::HeatingMode,
};

use crate::home_state::{
    TargetHeatingMode,
    calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

fn from_iso(s: &str) -> DateTime {
    DateTime::from_iso(s).expect("Invalid ISO datetime")
}

pub struct SetPointStateProvider;

impl DerivedStateProvider<SetPoint, DegreeCelsius> for SetPointStateProvider {
    fn calculate_current(&self, id: SetPoint, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        use crate::device_state::SetPoint as DeviceSetPoint;

        match id {
            SetPoint::RoomOfRequirements if from_iso("2025-11-22T15:08:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::RoomOfRequirements)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::LivingRoomBig if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::LivingRoom)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::LivingRoomSmall if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::LivingRoom)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Kitchen if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::Kitchen)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Bedroom if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::Bedroom)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::Bathroom if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::Bathroom)?;
                setpoint_for_mode(&id, &mode.value).into()
            }
            SetPoint::RoomOfRequirements => ctx.device_state(DeviceSetPoint::RoomOfRequirements).map(|dp| dp.value),
            SetPoint::LivingRoomBig => ctx.device_state(DeviceSetPoint::LivingRoomBig).map(|dp| dp.value),
            SetPoint::LivingRoomSmall => ctx.device_state(DeviceSetPoint::LivingRoomSmall).map(|dp| dp.value),
            SetPoint::Bedroom => ctx.device_state(DeviceSetPoint::Bedroom).map(|dp| dp.value),
            SetPoint::Kitchen => ctx.device_state(DeviceSetPoint::Kitchen).map(|dp| dp.value),
            SetPoint::Bathroom => ctx.device_state(DeviceSetPoint::Bathroom).map(|dp| dp.value),
        }
    }
}

fn setpoint_for_mode(id: &SetPoint, mode: &HeatingMode) -> DegreeCelsius {
    let t = match (id, mode) {
        (_, HeatingMode::Manual(t, _)) => t.0,
        (_, HeatingMode::Ventilation) => 0.0,
        (SetPoint::LivingRoomBig, HeatingMode::EnergySaving) => 19.0,
        (SetPoint::LivingRoomBig, HeatingMode::PostVentilation) => 19.0,
        (SetPoint::LivingRoomBig, HeatingMode::Sleep) => 18.5,
        (SetPoint::LivingRoomBig, HeatingMode::Comfort) => 19.5,
        (SetPoint::LivingRoomBig, HeatingMode::Away) => 17.0,
        (SetPoint::LivingRoomSmall, HeatingMode::EnergySaving) => 19.0,
        (SetPoint::LivingRoomSmall, HeatingMode::PostVentilation) => 19.0,
        (SetPoint::LivingRoomSmall, HeatingMode::Sleep) => 18.5,
        (SetPoint::LivingRoomSmall, HeatingMode::Comfort) => 19.5,
        (SetPoint::LivingRoomSmall, HeatingMode::Away) => 17.0,
        (SetPoint::RoomOfRequirements, HeatingMode::EnergySaving) => 18.0,
        (SetPoint::RoomOfRequirements, HeatingMode::PostVentilation) => 18.0,
        (SetPoint::RoomOfRequirements, HeatingMode::Sleep) => 17.0,
        (SetPoint::RoomOfRequirements, HeatingMode::Comfort) => 19.0,
        (SetPoint::RoomOfRequirements, HeatingMode::Away) => 16.0,
        (SetPoint::Bedroom, HeatingMode::EnergySaving) => 17.5,
        (SetPoint::Bedroom, HeatingMode::PostVentilation) => 17.0,
        (SetPoint::Bedroom, HeatingMode::Sleep) => 18.5,
        (SetPoint::Bedroom, HeatingMode::Comfort) => 19.0,
        (SetPoint::Bedroom, HeatingMode::Away) => 16.5,
        (SetPoint::Kitchen, HeatingMode::EnergySaving) => 17.0,
        (SetPoint::Kitchen, HeatingMode::PostVentilation) => 16.5,
        (SetPoint::Kitchen, HeatingMode::Sleep) => 16.5,
        (SetPoint::Kitchen, HeatingMode::Comfort) => 18.0,
        (SetPoint::Kitchen, HeatingMode::Away) => 16.0,
        (SetPoint::Bathroom, _) => 15.0,
    };

    DegreeCelsius(t)
}
