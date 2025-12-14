use r#macro::{EnumVariants, Id};

use crate::core::timeseries::DataPoint;
use crate::core::unit::Watt;
use crate::home::state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::home::state::{CurrentPowerUsage, PowerAvailable};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum IsRunning {
    LivingRoomTv,
    RoomOfRequirementsMonitor,
}

pub struct IsRunningStateProvider;

impl DerivedStateProvider<IsRunning, bool> for IsRunningStateProvider {
    fn calculate_current(&self, id: IsRunning, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        match id {
            IsRunning::LivingRoomTv => ctx.get(PowerAvailable::LivingRoomTv),
            IsRunning::RoomOfRequirementsMonitor => {
                let power_usage_dp = ctx.get(CurrentPowerUsage::RoomOfRequirementsMonitor)?;
                Some(DataPoint::new(power_usage_dp.value > Watt(15.0), power_usage_dp.timestamp))
            }
        }
    }
}
