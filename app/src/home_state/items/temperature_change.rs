use r#macro::{EnumVariants, Id};

use crate::automation::Thermostat;
use crate::core::unit::{DegreeCelsius, RateOfChange};
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TemperatureChange {
    Radiator(Thermostat),
}

pub struct TemperatureChangeStateProvider;

impl DerivedStateProvider<TemperatureChange, RateOfChange<DegreeCelsius>> for TemperatureChangeStateProvider {
    fn calculate_current(
        &self,
        id: TemperatureChange,
        ctx: &StateCalculationContext,
    ) -> Option<RateOfChange<DegreeCelsius>> {
        let thermostat = match id {
            TemperatureChange::Radiator(thermostat) => thermostat,
        };
        let temperatures = ctx.all_since(Temperature::Radiator(thermostat), t!(2 hours ago))?;
        let (prev, next) = temperatures.last2()?;
        Some(RateOfChange::from_dps(prev, next))
    }
}
