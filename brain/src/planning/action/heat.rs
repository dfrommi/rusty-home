use anyhow::Result;
use api::{
    command::{Command, Thermostat},
    state::SetPoint,
};
use goap::{Effects, Preconditions};
use support::unit::DegreeCelsius;

use crate::{
    planning::{
        goal::room_comfort::RoomComfortLevel, BedroomState, HomeState, KitchenState,
        LivingRoomState, RoomOfRequirementsState,
    },
    prelude::{DataPointAccess, UserControlled},
    thing::Executable,
};

use super::Action;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Heat {
    LivingRoom(RoomComfortLevel),
    Bedroom(RoomComfortLevel),
    Kitchen(RoomComfortLevel),
    RoomOfRequirements(RoomComfortLevel),
}

impl Heat {
    fn room_comfort_level(&self) -> &RoomComfortLevel {
        match self {
            Heat::LivingRoom(room_comfort_level)
            | Heat::Bedroom(room_comfort_level)
            | Heat::Kitchen(room_comfort_level)
            | Heat::RoomOfRequirements(room_comfort_level) => room_comfort_level,
        }
    }

    fn set_point(room_comfort_level: &RoomComfortLevel) -> DegreeCelsius {
        match room_comfort_level {
            RoomComfortLevel::EnergySaving => DegreeCelsius(15.0),
            RoomComfortLevel::Normal => DegreeCelsius(19.0),
            RoomComfortLevel::Comfortable => DegreeCelsius(20.0),
        }
    }

    fn thermostat(&self) -> Thermostat {
        match self {
            Heat::LivingRoom(_) => Thermostat::LivingRoom,
            Heat::Bedroom(_) => Thermostat::Bedroom,
            Heat::Kitchen(_) => Thermostat::Kitchen,
            Heat::RoomOfRequirements(_) => Thermostat::RoomOfRequirements,
        }
    }
}

impl Action for Heat {
    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: Self::set_point(self.room_comfort_level()),
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.thermostat(),
            target_state: api::command::HeatingTargetState::Off,
        }
        .execute()
        .await
    }

    async fn is_running(&self) -> bool {
        let current_set_point = match self {
            Heat::LivingRoom(_) => SetPoint::LivingRoom,
            Heat::Bedroom(_) => SetPoint::Bedroom,
            Heat::Kitchen(_) => SetPoint::Kitchen,
            Heat::RoomOfRequirements(_) => SetPoint::RoomOfRequirements,
        }
        .current()
        .await;

        current_set_point.map_or(false, |current| {
            current == Self::set_point(self.room_comfort_level())
        })
    }

    async fn is_enabled(&self) -> bool {
        let result = match self {
            Heat::LivingRoom(_) => UserControlled::LivingRoomThermostat,
            Heat::Bedroom(_) => UserControlled::BedroomThermostat,
            Heat::Kitchen(_) => UserControlled::KitchenThermostat,
            Heat::RoomOfRequirements(_) => UserControlled::RoomOfRequirementsThermostat,
        }
        .current()
        .await;

        result.map_or(true, |user| !user)
    }
}

impl Preconditions<HomeState> for Heat {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        let (target_level, temperature) = match self {
            Heat::LivingRoom(room_comfort_level) => {
                (room_comfort_level, state.living_room.temperature)
            }
            Heat::Bedroom(room_comfort_level) => (room_comfort_level, state.bedroom.temperature),
            Heat::Kitchen(room_comfort_level) => (room_comfort_level, state.kitchen.temperature),
            Heat::RoomOfRequirements(room_comfort_level) => {
                (room_comfort_level, state.room_of_requirements.temperature)
            }
        };

        RoomComfortLevel::from_temperature(temperature) == *target_level
    }
}

impl Effects<HomeState> for Heat {
    fn apply_to(&self, state: &HomeState) -> HomeState {
        match self {
            Heat::LivingRoom(room_comfort_level) => HomeState {
                living_room: LivingRoomState {
                    temperature: Self::set_point(room_comfort_level),
                    ..state.living_room
                },
                ..state.clone()
            },
            Heat::Bedroom(room_comfort_level) => HomeState {
                bedroom: BedroomState {
                    temperature: Self::set_point(room_comfort_level),
                    ..state.bedroom
                },
                ..state.clone()
            },
            Heat::Kitchen(room_comfort_level) => HomeState {
                kitchen: KitchenState {
                    temperature: Self::set_point(room_comfort_level),
                    ..state.kitchen
                },
                ..state.clone()
            },
            Heat::RoomOfRequirements(room_comfort_level) => HomeState {
                room_of_requirements: RoomOfRequirementsState {
                    temperature: Self::set_point(room_comfort_level),
                    ..state.room_of_requirements
                },
                ..state.clone()
            },
        }
    }
}
