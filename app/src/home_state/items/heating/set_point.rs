use r#macro::{EnumVariants, Id};

use crate::{
    automation::{HeatingZone, Radiator},
    core::{range::Range, unit::DegreeCelsius},
    home_state::items::from_iso,
};

use crate::home_state::{
    TargetHeatingMode,
    calc::{DerivedStateProvider, StateCalculationContext},
};

use super::setpoint_for_mode;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    Target(Radiator),
    Current(Radiator),
}

pub struct SetPointStateProvider;

impl DerivedStateProvider<SetPoint, Range<DegreeCelsius>> for SetPointStateProvider {
    fn calculate_current(&self, id: SetPoint, ctx: &StateCalculationContext) -> Option<Range<DegreeCelsius>> {
        match id {
            SetPoint::Target(Radiator::RoomOfRequirements) if from_iso("2025-11-22T15:08:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements))?;
                setpoint_for_mode(Radiator::RoomOfRequirements, &mode.value).into()
            }
            SetPoint::Target(Radiator::LivingRoomBig) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))?;
                setpoint_for_mode(Radiator::LivingRoomBig, &mode.value).into()
            }
            SetPoint::Target(Radiator::LivingRoomSmall) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))?;
                setpoint_for_mode(Radiator::LivingRoomSmall, &mode.value).into()
            }
            SetPoint::Target(Radiator::Kitchen) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Kitchen))?;
                setpoint_for_mode(Radiator::Kitchen, &mode.value).into()
            }
            SetPoint::Target(Radiator::Bedroom) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))?;
                setpoint_for_mode(Radiator::Bedroom, &mode.value).into()
            }
            SetPoint::Target(Radiator::Bathroom) if from_iso("2025-12-21T18:00:00+00:00").is_passed() => {
                let mode = ctx.get(TargetHeatingMode::HeatingZone(HeatingZone::Bathroom))?;
                setpoint_for_mode(Radiator::Bathroom, &mode.value).into()
            }
            SetPoint::Target(radiator) | SetPoint::Current(radiator) => setpoint_for_device_reading(radiator, ctx),
        }
    }
}

fn setpoint_for_device_reading(radiator: Radiator, ctx: &StateCalculationContext) -> Option<Range<DegreeCelsius>> {
    use crate::device_state::SetPoint as DeviceSetPoint;

    let (min, max) = match radiator {
        Radiator::LivingRoomBig => (DeviceSetPoint::LivingRoomBigLower, DeviceSetPoint::LivingRoomBig),
        Radiator::LivingRoomSmall => (DeviceSetPoint::LivingRoomSmallLower, DeviceSetPoint::LivingRoomSmall),
        Radiator::Bedroom => (DeviceSetPoint::BedroomLower, DeviceSetPoint::Bedroom),
        Radiator::Kitchen => (DeviceSetPoint::KitchenLower, DeviceSetPoint::Kitchen),
        Radiator::RoomOfRequirements => (DeviceSetPoint::RoomOfRequirementsLower, DeviceSetPoint::RoomOfRequirements),
        Radiator::Bathroom => (DeviceSetPoint::BathroomLower, DeviceSetPoint::Bathroom),
    };

    let min_value = ctx.device_state(min).map(|dp| dp.value.0);
    let max_value = ctx.device_state(max).map(|dp| dp.value.0);

    match (min_value, max_value) {
        (Some(min), Some(max)) => Some(Range::new(DegreeCelsius(min), DegreeCelsius(max))),
        (Some(t), None) | (None, Some(t)) => Some(Range::new(DegreeCelsius(t), DegreeCelsius(t))),
        (None, None) => None,
    }
}
