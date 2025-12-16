use r#macro::{EnumVariants, Id};

use crate::{
    automation::HeatingZone,
    core::{time::DateTime, timeseries::DataPoint, unit::DegreeCelsius},
};

use crate::home_state::{
    TargetHeatingMode,
    calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
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
    fn calculate_current(&self, id: SetPoint, ctx: &StateCalculationContext) -> Option<DataPoint<DegreeCelsius>> {
        use crate::device_state::SetPoint as DeviceSetPoint;

        match id {
            SetPoint::RoomOfRequirements if from_iso("2025-11-22T15:08:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::RoomOfRequirements)?;
                let value = HeatingZone::RoomOfRequirements.setpoint_for_mode(&mode.value);
                Some(DataPoint::new(value, mode.timestamp))
            }
            SetPoint::RoomOfRequirements => ctx.device_state(DeviceSetPoint::RoomOfRequirements),
            SetPoint::LivingRoomBig => ctx.device_state(DeviceSetPoint::LivingRoomBig),
            SetPoint::LivingRoomSmall => ctx.device_state(DeviceSetPoint::LivingRoomSmall),
            SetPoint::Bedroom => ctx.device_state(DeviceSetPoint::Bedroom),
            SetPoint::Kitchen => ctx.device_state(DeviceSetPoint::Kitchen),
            SetPoint::Bathroom => ctx.device_state(DeviceSetPoint::Bathroom),
        }
    }
}
