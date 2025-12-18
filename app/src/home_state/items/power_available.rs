use r#macro::{EnumVariants, Id};

use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum PowerAvailable {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}

pub struct PowerAvailableStateProvider;

impl DerivedStateProvider<PowerAvailable, bool> for PowerAvailableStateProvider {
    fn calculate_current(
        &self,
        id: PowerAvailable,
        ctx: &StateCalculationContext,
    ) -> Option<crate::core::timeseries::DataPoint<bool>> {
        use crate::device_state::PowerAvailable as DevicePowerAvailable;

        ctx.device_state(match id {
            PowerAvailable::Dehumidifier => DevicePowerAvailable::Dehumidifier,
            PowerAvailable::LivingRoomNotificationLight => DevicePowerAvailable::LivingRoomNotificationLight,
            PowerAvailable::InfraredHeater => DevicePowerAvailable::InfraredHeater,
            PowerAvailable::LivingRoomTv => DevicePowerAvailable::LivingRoomTv,
        })
    }
}
