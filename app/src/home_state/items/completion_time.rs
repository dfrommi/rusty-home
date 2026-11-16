use r#macro::{EnumVariants, Id};

use crate::core::time::DateTime;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum CompletionTime {
    WashingMachineEstimated,
}

pub struct CompletionTimeStateProvider;

impl DerivedStateProvider<CompletionTime, DateTime> for CompletionTimeStateProvider {
    fn calculate_current(&self, id: CompletionTime, ctx: &StateCalculationContext) -> Option<DateTime> {
        use crate::device_state::CompletionTime as DeviceCompletionTime;

        match id {
            CompletionTime::WashingMachineEstimated => ctx
                .device_state(DeviceCompletionTime::WashingMachineEstimated)
                .map(|dp| dp.value),
        }
    }
}
