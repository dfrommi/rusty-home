use r#macro::{EnumVariants, Id};

use crate::{
    core::unit::FanAirflow,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum FanActivity {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
    BedroomDehumidifier,
}

pub struct FanActivityStateProvider;

impl DerivedStateProvider<FanActivity, FanAirflow> for FanActivityStateProvider {
    fn calculate_current(&self, id: FanActivity, ctx: &StateCalculationContext) -> Option<FanAirflow> {
        use crate::device_state::FanActivity as DeviceFanActivity;

        ctx.device_state(match id {
            FanActivity::LivingRoomCeilingFan => DeviceFanActivity::LivingRoomCeilingFan,
            FanActivity::BedroomCeilingFan => DeviceFanActivity::BedroomCeilingFan,
            FanActivity::BedroomDehumidifier => DeviceFanActivity::BedroomDehumidifier,
        })
        .map(|dp| dp.value)
    }
}
