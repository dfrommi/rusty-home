use r#macro::{EnumVariants, Id};

use crate::{
    automation::HeatingZone,
    core::{time::DateTime, unit::DegreeCelsius},
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
                let value = HeatingZone::RoomOfRequirements.setpoint_for_mode(&mode.value);
                Some(value)
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
