mod follow_heating_schedule;
mod ir_heater_auto_turn_off;
mod provide_ambient_temperature;
mod support_ventilation_with_fan;

use crate::{
    core::unit::DegreeCelsius,
    home::command::{HeatingTargetState, Thermostat},
};

pub use follow_heating_schedule::FollowHeatingSchedule;
pub use ir_heater_auto_turn_off::IrHeaterAutoTurnOff;
pub use provide_ambient_temperature::ProvideAmbientTemperature;
pub use support_ventilation_with_fan::SupportVentilationWithFan;

use super::*;

#[derive(Debug, Clone, derive_more::Display)]
pub enum HeatingZone {
    #[display("LivingRoom")]
    LivingRoom,
    #[display("Bedroom")]
    Bedroom,
    #[display("Kitchen")]
    Kitchen,
    #[display("RoomOfRequirements")]
    RoomOfRequirements,
    #[display("Bathroom")]
    Bathroom,
}

impl HeatingZone {
    pub fn thermostat(&self) -> Thermostat {
        match self {
            HeatingZone::LivingRoom => Thermostat::LivingRoom,
            HeatingZone::Bedroom => Thermostat::Bedroom,
            HeatingZone::Kitchen => Thermostat::Kitchen,
            HeatingZone::RoomOfRequirements => Thermostat::RoomOfRequirements,
            HeatingZone::Bathroom => Thermostat::Bathroom,
        }
    }
}

impl HeatingTargetState {
    pub fn for_mode(mode: &HeatingMode, thermostat: &Thermostat) -> Self {
        let default_temperature = match thermostat {
            Thermostat::LivingRoom => DegreeCelsius(19.0),
            Thermostat::Bedroom => DegreeCelsius(19.0),
            Thermostat::Kitchen => DegreeCelsius(17.0),
            Thermostat::RoomOfRequirements => DegreeCelsius(18.0),
            Thermostat::Bathroom => DegreeCelsius(15.0),
        };

        //TODO Room specific
        match mode {
            HeatingMode::Ventilation | HeatingMode::PostVentilation => HeatingTargetState::WindowOpen,
            HeatingMode::EnergySaving => HeatingTargetState::Heat {
                temperature: default_temperature,
                duration: t!(1 hours),
            },
            HeatingMode::Comfort => HeatingTargetState::Heat {
                temperature: default_temperature + DegreeCelsius(1.0),
                duration: t!(1 hours),
            },
            HeatingMode::Sleep => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(1.0),
                duration: t!(1 hours),
            },
            HeatingMode::Away => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(2.0),
                duration: t!(1 hours),
            },
        }
    }
}
