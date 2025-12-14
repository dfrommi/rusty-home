use r#macro::{EnumVariants, Id};

use crate::{
    core::{timeseries::DataPoint, unit::DegreeCelsius},
    home::{
        HeatingZone,
        state::{TargetHeatingMode, calc::StateCalculationContext},
    },
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

impl SetPoint {
    pub fn get_derived_setpoint(&self, ctx: &StateCalculationContext) -> Option<DataPoint<DegreeCelsius>> {
        //TODO temp workaround for migration
        match self {
            SetPoint::RoomOfRequirements => {
                let mode = ctx.get(TargetHeatingMode::RoomOfRequirements)?;
                let value = HeatingZone::RoomOfRequirements.setpoint_for_mode(&mode.value);
                Some(DataPoint::new(value, mode.timestamp))
            }

            _ => None,
        }
    }
}
