use r#macro::{EnumVariants, Id};

use crate::{
    core::unit::MicrogramsPerCubicMeter,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ParticulateMatter {
    LivingRoomPM25,
}

pub struct ParticulateMatterStateProvider;

impl DerivedStateProvider<ParticulateMatter, MicrogramsPerCubicMeter> for ParticulateMatterStateProvider {
    fn calculate_current(
        &self,
        id: ParticulateMatter,
        ctx: &StateCalculationContext,
    ) -> Option<MicrogramsPerCubicMeter> {
        use crate::device_state::ParticulateMatter as DeviceParticulateMatter;

        ctx.device_state(match id {
            ParticulateMatter::LivingRoomPM25 => DeviceParticulateMatter::LivingRoomPM25,
        })
        .map(|dp| dp.value)
    }
}
