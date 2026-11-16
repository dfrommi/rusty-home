use r#macro::{EnumVariants, Id};

use crate::core::unit::Watt;
use crate::device_state::{CurrentPowerUsage, IsRunning as DeviceIsRunning};
use crate::home_state::PowerAvailable;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum IsRunning {
    LivingRoomTv,
    RoomOfRequirementsMonitor,
    WashingMachine,
}

pub struct IsRunningStateProvider;

impl DerivedStateProvider<IsRunning, bool> for IsRunningStateProvider {
    fn calculate_current(&self, id: IsRunning, ctx: &StateCalculationContext) -> Option<bool> {
        match id {
            IsRunning::LivingRoomTv => ctx.get(PowerAvailable::LivingRoomTv).map(|dp| dp.value),
            IsRunning::RoomOfRequirementsMonitor => {
                let power_usage_dp = ctx.device_state(CurrentPowerUsage::RoomOfRequirementsMonitor)?;
                Some(power_usage_dp.value > Watt(15.0))
            }
            IsRunning::WashingMachine => ctx.device_state(DeviceIsRunning::WashingMachine).map(|dp| dp.value),
        }
    }
}
