use r#macro::{EnumVariants, Id};

use crate::{
    core::timeseries::DataPoint,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergySaving {
    LivingRoomTv,
}

pub struct EnergySavingStateProvider;

impl DerivedStateProvider<EnergySaving, bool> for EnergySavingStateProvider {
    fn calculate_current(&self, id: EnergySaving, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        use crate::device_state::EnergySaving as DeviceEnergySaving;

        ctx.device_state(match id {
            EnergySaving::LivingRoomTv => DeviceEnergySaving::LivingRoomTv,
        })
    }
}
