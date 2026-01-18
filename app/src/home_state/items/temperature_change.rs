use r#macro::{EnumVariants, Id};

use crate::automation::{Radiator, Room};
use crate::core::unit::{DegreeCelsius, RateOfChange};
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TemperatureChange {
    Radiator(Radiator),
    Room(Room),
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
            TemperatureChange::Room(room) => Temperature::Room(room),
        };
        let temperatures = ctx.all_since(temp_item, t!(2 hours ago))?;
        temperatures.last_change(t!(5 minutes))
    }
}
