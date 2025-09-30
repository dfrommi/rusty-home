use crate::{
    core::unit::DegreeCelsius,
    home::{
        command::{HeatingTargetState, Thermostat},
        state::HeatingMode,
    },
};

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
    pub fn thermostats(&self) -> &[Thermostat] {
        match self {
            HeatingZone::LivingRoom => &[Thermostat::LivingRoomBig, Thermostat::LivingRoomSmall],
            HeatingZone::Bedroom => &[Thermostat::Bedroom],
            HeatingZone::Kitchen => &[Thermostat::Kitchen],
            HeatingZone::RoomOfRequirements => &[Thermostat::RoomOfRequirements],
            HeatingZone::Bathroom => &[Thermostat::Bathroom],
        }
    }

    pub fn default_setpoint(&self) -> DegreeCelsius {
        match self {
            HeatingZone::LivingRoom => DegreeCelsius(19.0),
            HeatingZone::Bedroom => DegreeCelsius(19.0),
            HeatingZone::Kitchen => DegreeCelsius(17.0),
            HeatingZone::RoomOfRequirements => DegreeCelsius(18.0),
            HeatingZone::Bathroom => DegreeCelsius(15.0),
        }
    }

    pub fn heating_state(&self, mode: &HeatingMode) -> HeatingTargetState {
        let default_temperature = self.default_setpoint();
        //TODO Room specific
        match mode {
            HeatingMode::Ventilation | HeatingMode::PostVentilation => HeatingTargetState::WindowOpen,
            HeatingMode::EnergySaving => HeatingTargetState::Heat {
                temperature: default_temperature,
            },
            HeatingMode::Comfort => HeatingTargetState::Heat {
                temperature: default_temperature + DegreeCelsius(1.0),
            },
            HeatingMode::Sleep => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(1.0),
            },
            HeatingMode::Away => HeatingTargetState::Heat {
                temperature: default_temperature - DegreeCelsius(2.0),
            },
        }
    }

    pub fn for_thermostat(thermostat: &Thermostat) -> Self {
        match thermostat {
            Thermostat::LivingRoomBig | Thermostat::LivingRoomSmall => HeatingZone::LivingRoom,
            Thermostat::Bedroom => HeatingZone::Bedroom,
            Thermostat::Kitchen => HeatingZone::Kitchen,
            Thermostat::RoomOfRequirements => HeatingZone::RoomOfRequirements,
            Thermostat::Bathroom => HeatingZone::Bathroom,
        }
    }
}
