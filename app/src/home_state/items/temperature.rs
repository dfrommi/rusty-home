use r#macro::{EnumVariants, Id};

use crate::core::domain::{Radiator, Room};
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
    BedroomCorner,
}

// Thermal-bridge temperature factor for the mould-prone bedroom corner.
//
// TODO calibrate empirically with a one-off IR thermometer reading on a
// cold day: `f_Rsi = (T_surface - T_outside) / (T_room - T_outside)`.
// Current value is a conservative estimate for an old-building exterior corner.
const BEDROOM_CORNER_F_RSI: f64 = 0.55;

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
            Temperature::BedroomCorner => {
                // Estimated surface temperature of the mould-prone upper corner in the
                // bedroom (exterior wall). There is no physical sensor at this spot — and
                // installing one would be invalidated by the dehumidifier's air drift —
                // so the value is derived from indoor/outdoor air temperature via a
                // thermal-bridge model:
                //
                //     T_surface = T_outside + f_Rsi * (T_room - T_outside)
                //
                // `f_Rsi` is the temperature factor of the corner (DIN 4108-2 / ISO 13788).
                // A perfectly insulated wall would have `f_Rsi = 1.0`; the German code
                // minimum for new buildings is 0.7; old-building exterior corners are
                // typically 0.55-0.65.
                let t_room = ctx.get(Temperature::Room(Room::Bedroom))?.value;
                let t_outside = ctx.get(Temperature::Outside)?.value;
                let t_surface = t_outside.0 + BEDROOM_CORNER_F_RSI * (t_room.0 - t_outside.0);

                ctx.trace(id, "t_room", t_room);
                ctx.trace(id, "t_outside", t_outside);
                ctx.trace(id, "f_rsi", BEDROOM_CORNER_F_RSI);

                DegreeCelsius(t_surface)
            }
        }
        .into()
    }
}
