use crate::core::unit::RawValue;
use crate::home_state::RawVendorValue;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::{
    core::{timeseries::DataPoint, unit::Percent},
    home::Thermostat,
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Load {
    Thermostat(Thermostat),
}

pub struct LoadStateProvider;

impl DerivedStateProvider<Load, Percent> for LoadStateProvider {
    fn calculate_current(&self, id: Load, ctx: &StateCalculationContext) -> Option<DataPoint<Percent>> {
        match id {
            Load::Thermostat(thermostat) => {
                let raw = ctx.get(RawVendorValue::AllyLoadEstimate(thermostat.clone()))?;
                Some(DataPoint::new(percent_load_for_ally(raw.value), raw.timestamp))
            }
        }
    }
}

fn percent_load_for_ally(raw_value: RawValue) -> Percent {
    // Range: discard < -500, max value 3600, below 0 are different levels of zero.
    // -8000 invalid
    // TODO skip lower than -500 instead of mapping to 0?
    Percent(raw_value.0.max(0.0) / 36.0) // 0-3600 to percent
}
