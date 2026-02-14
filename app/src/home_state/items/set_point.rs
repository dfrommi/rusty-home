use r#macro::{EnumVariants, Id};

use crate::{
    automation::{HeatingZone, Radiator},
    core::{range::Range, time::DateTime, unit::DegreeCelsius},
    home_state::{HeatingMode, items::from_iso},
};

use crate::home_state::{
    TargetHeatingMode,
    calc::{DerivedStateProvider, StateCalculationContext},
};

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

fn setpoint_for_mode(radiator: Radiator, mode: &HeatingMode) -> Range<DegreeCelsius> {
    let t = match (radiator, mode) {
        (_, HeatingMode::Manual(t, _)) => t.0,
        (_, HeatingMode::Ventilation) => 0.0,
        (Radiator::LivingRoomBig, HeatingMode::EnergySaving) => 19.0,
        (Radiator::LivingRoomBig, HeatingMode::PostVentilation) => 19.0,
        (Radiator::LivingRoomBig, HeatingMode::Sleep) => 18.5,
        (Radiator::LivingRoomBig, HeatingMode::Comfort) => 19.5,
        (Radiator::LivingRoomBig, HeatingMode::Away) => 17.0,
        (Radiator::LivingRoomSmall, HeatingMode::EnergySaving) => 19.0,
        (Radiator::LivingRoomSmall, HeatingMode::PostVentilation) => 19.0,
        (Radiator::LivingRoomSmall, HeatingMode::Sleep) => 18.5,
        (Radiator::LivingRoomSmall, HeatingMode::Comfort) => 19.5,
        (Radiator::LivingRoomSmall, HeatingMode::Away) => 17.0,
        (Radiator::RoomOfRequirements, HeatingMode::EnergySaving) => 18.0,
        (Radiator::RoomOfRequirements, HeatingMode::PostVentilation) => 18.0,
        (Radiator::RoomOfRequirements, HeatingMode::Sleep) => 17.0,
        (Radiator::RoomOfRequirements, HeatingMode::Comfort) => 19.0,
        (Radiator::RoomOfRequirements, HeatingMode::Away) => 16.0,
        (Radiator::Bedroom, HeatingMode::EnergySaving) => 17.5,
        (Radiator::Bedroom, HeatingMode::PostVentilation) => 17.0,
        (Radiator::Bedroom, HeatingMode::Sleep) => 18.5,
        (Radiator::Bedroom, HeatingMode::Comfort) => 19.0,
        (Radiator::Bedroom, HeatingMode::Away) => 16.5,
        (Radiator::Kitchen, HeatingMode::EnergySaving) => 17.0,
        (Radiator::Kitchen, HeatingMode::PostVentilation) => 16.5,
        (Radiator::Kitchen, HeatingMode::Sleep) => 16.5,
        (Radiator::Kitchen, HeatingMode::Comfort) => 18.0,
        (Radiator::Kitchen, HeatingMode::Away) => 16.0,
        (Radiator::Bathroom, _) => 15.0,
    };

    //range: 0.2 - 1.0 with 0.2 increments
    let offset = match mode {
        HeatingMode::Comfort | HeatingMode::Manual(_, _) => 0.4,
        HeatingMode::EnergySaving
        | HeatingMode::PostVentilation
        | HeatingMode::Ventilation
        | HeatingMode::Sleep
        | HeatingMode::Away => 1.0,
    };

    Range::new(DegreeCelsius(t), DegreeCelsius(t - offset))
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
