use r#macro::{EnumVariants, Id};

use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum EnergySaving {
    LivingRoomTv,
}

pub struct EnergySavingStateProvider;

impl DerivedStateProvider<EnergySaving, bool> for EnergySavingStateProvider {
    fn calculate_current(&self, id: EnergySaving, ctx: &StateCalculationContext) -> Option<bool> {
        use crate::device_state::EnergySaving as DeviceEnergySaving;

        ctx.device_state(match id {
            EnergySaving::LivingRoomTv => DeviceEnergySaving::LivingRoomTv,
        })
        .map(|dp| dp.value)
    }
}
