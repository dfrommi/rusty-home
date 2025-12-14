use r#macro::{EnumVariants, Id};

use crate::{
    core::unit::RawValue,
    home::Thermostat,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RawVendorValue {
    AllyLoadEstimate(Thermostat),
    AllyLoadMean(Thermostat),
}

pub struct RawVendorValueStateProvider;

impl DerivedStateProvider<RawVendorValue, RawValue> for RawVendorValueStateProvider {
    fn calculate_current(
        &self,
        id: RawVendorValue,
        ctx: &StateCalculationContext,
    ) -> Option<crate::core::timeseries::DataPoint<RawValue>> {
        use crate::device_state::RawVendorValue as DeviceRawVendorValue;

        ctx.device_state(match id {
            RawVendorValue::AllyLoadEstimate(thermostat) => DeviceRawVendorValue::AllyLoadEstimate(thermostat),
            RawVendorValue::AllyLoadMean(thermostat) => DeviceRawVendorValue::AllyLoadMean(thermostat),
        })
    }
}
