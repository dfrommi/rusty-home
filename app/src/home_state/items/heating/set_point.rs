use r#macro::{EnumVariants, Id};

use crate::{
    core::domain::Radiator,
    core::{range::Range, unit::DegreeCelsius},
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
            SetPoint::Target(radiator) => {
                let mode = ctx.get(TargetHeatingMode::from_radiator(radiator))?;
                Some(setpoint_for_mode(radiator, &mode.value))
            }
            SetPoint::Current(radiator) => setpoint_for_device_reading(radiator, ctx),
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
