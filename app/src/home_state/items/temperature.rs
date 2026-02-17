use r#macro::{EnumVariants, Id};

use crate::automation::{Radiator, Room};
use crate::core::unit::DegreeCelsius;
use crate::home_state::TemperatureChange;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Temperature {
    Outside,
    Room(Room),
    Radiator(Radiator),
    RadiatorExternalTempSensor(Radiator),
    RadiatorIn15Minutes(Radiator),
    RoomIn15Minutes(Room),
}

pub struct TemperatureStateProvider;

impl DerivedStateProvider<Temperature, DegreeCelsius> for TemperatureStateProvider {
    fn calculate_current(&self, id: Temperature, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        use crate::device_state::Temperature as DeviceTemperature;

        match id {
            Temperature::Outside => ctx.device_state(DeviceTemperature::Outside)?.value,
            Temperature::Room(room) => match room {
                Room::LivingRoom => ctx.device_state(DeviceTemperature::LivingRoomTado)?.value,
                Room::RoomOfRequirements => ctx.device_state(DeviceTemperature::RoomOfRequirementsTado)?.value,
                Room::Bedroom => ctx.device_state(DeviceTemperature::BedroomTado)?.value,
                Room::Kitchen => ctx.device_state(DeviceTemperature::Kitchen)?.value,
                Room::Bathroom => {
                    let shower = ctx.device_state(DeviceTemperature::BathroomShower);
                    let dehumidifier = ctx.device_state(DeviceTemperature::Dehumidifier);

                    match (shower, dehumidifier) {
                        (Some(shower), Some(dehumidifier)) => {
                            DegreeCelsius((shower.value.0 + dehumidifier.value.0) / 2.0)
                        }
                        (Some(shower), None) => shower.value,
                        (None, Some(dehumidifier)) => dehumidifier.value,
                        (None, None) => return None,
                    }
                }
            },
            Temperature::Radiator(thermostat) => ctx.device_state(DeviceTemperature::Radiator(thermostat))?.value,
            Temperature::RadiatorExternalTempSensor(radiator) => {
                ctx.device_state(DeviceTemperature::ThermostatExternalInput(radiator))?
                    .value
            }
            Temperature::RadiatorIn15Minutes(thermostat) => {
                let current = ctx.device_state(DeviceTemperature::Radiator(thermostat))?.value;
                let change = ctx.get(TemperatureChange::Radiator(thermostat))?.value;
                current + change.per(t!(15 minutes))
            }
            Temperature::RoomIn15Minutes(room) => {
                let current = ctx.get(Temperature::Room(room))?.value;
                let change = ctx.get(TemperatureChange::Room(room))?.value;
                current + change.per(t!(15 minutes))
            }
        }
        .into()
    }
}
