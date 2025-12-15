use r#macro::{EnumVariants, Id};

use crate::automation::Thermostat;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RawVendorValue {
    AllyLoadEstimate(Thermostat),
    AllyLoadMean(Thermostat),
}
