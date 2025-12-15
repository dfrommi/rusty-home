use r#macro::{EnumVariants, Id};

use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::{automation::Thermostat, core::unit::DegreeCelsius};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    Kitchen,
    Bathroom,
    ThermostatExternal(Thermostat),
}

pub struct TemperatureStateProvider;

impl DerivedStateProvider<Temperature, DegreeCelsius> for TemperatureStateProvider {
    fn calculate_current(
        &self,
        id: Temperature,
        ctx: &StateCalculationContext,
    ) -> Option<crate::core::timeseries::DataPoint<DegreeCelsius>> {
        use crate::device_state::Temperature as DeviceTemperature;

        ctx.device_state(match id {
            Temperature::Outside => DeviceTemperature::Outside,
            Temperature::LivingRoom => DeviceTemperature::LivingRoomTado,
            Temperature::RoomOfRequirements => DeviceTemperature::RoomOfRequirementsTado,
            Temperature::Bedroom => DeviceTemperature::BedroomTado,
            Temperature::Kitchen => DeviceTemperature::Kitchen,
            Temperature::Bathroom => DeviceTemperature::BathroomShower,
            Temperature::ThermostatExternal(thermostat) => DeviceTemperature::ThermostatExternal(thermostat),
        })
    }
}
