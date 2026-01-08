use r#macro::{EnumVariants, Id};

use crate::automation::Thermostat;
use crate::core::unit::DegreeCelsius;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    Kitchen,
    Bathroom,
    Radiator(Thermostat),
}

pub struct TemperatureStateProvider;

impl DerivedStateProvider<Temperature, DegreeCelsius> for TemperatureStateProvider {
    fn calculate_current(&self, id: Temperature, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        use crate::device_state::Temperature as DeviceTemperature;

        match id {
            Temperature::Outside => ctx.device_state(DeviceTemperature::Outside)?.value,
            Temperature::LivingRoom => ctx.device_state(DeviceTemperature::LivingRoomTado)?.value,
            Temperature::RoomOfRequirements => ctx.device_state(DeviceTemperature::RoomOfRequirementsTado)?.value,
            Temperature::Bedroom => ctx.device_state(DeviceTemperature::BedroomTado)?.value,
            Temperature::Kitchen => ctx.device_state(DeviceTemperature::Kitchen)?.value,
            Temperature::Bathroom => {
                let shower = ctx.device_state(DeviceTemperature::BathroomShower);
                let dehumidifier = ctx.device_state(DeviceTemperature::Dehumidifier);

                match (shower, dehumidifier) {
                    (Some(shower), Some(dehumidifier)) => DegreeCelsius((shower.value.0 + dehumidifier.value.0) / 2.0),
                    (Some(shower), None) => shower.value,
                    (None, Some(dehumidifier)) => dehumidifier.value,
                    (None, None) => return None,
                }
            }
            Temperature::Radiator(thermostat) => ctx.device_state(DeviceTemperature::Radiator(thermostat))?.value,
        }
        .into()
    }
}
