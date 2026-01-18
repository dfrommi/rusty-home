use r#macro::{EnumVariants, Id};

use crate::automation::{HeatingZone, Radiator};
use crate::core::unit::{DegreeCelsius, RateOfChange};
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TemperatureChange {
    Radiator(Radiator),
    HeatingZone(HeatingZone),
}

pub struct TemperatureChangeStateProvider;

impl DerivedStateProvider<TemperatureChange, RateOfChange<DegreeCelsius>> for TemperatureChangeStateProvider {
    fn calculate_current(
        &self,
        id: TemperatureChange,
        ctx: &StateCalculationContext,
    ) -> Option<RateOfChange<DegreeCelsius>> {
        let temp_item = match id {
            TemperatureChange::Radiator(thermostat) => Temperature::Radiator(thermostat),
            TemperatureChange::HeatingZone(heating_zone) => heating_zone.inside_temperature(),
        };
        let temperatures = ctx.all_since(temp_item, t!(2 hours ago))?;
        temperatures.last_change(t!(5 minutes))
    }
}
