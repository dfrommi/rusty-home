use r#macro::{EnumVariants, Id};

use crate::{
    core::unit::AllergenIndexValue,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AllergenIndex {
    LivingRoom,
}

pub struct AllergenIndexStateProvider;

impl DerivedStateProvider<AllergenIndex, AllergenIndexValue> for AllergenIndexStateProvider {
    fn calculate_current(&self, id: AllergenIndex, ctx: &StateCalculationContext) -> Option<AllergenIndexValue> {
        use crate::device_state::AllergenIndex as DeviceAllergenIndex;

        ctx.device_state(match id {
            AllergenIndex::LivingRoom => DeviceAllergenIndex::LivingRoom,
        })
        .map(|dp| dp.value)
    }
}
