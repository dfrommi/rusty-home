use goap::Preconditions;
use support::unit::DegreeCelsius;

use crate::planning::HomeState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoomComfortLevel {
    EnergySaving,
    Normal,
    Comfortable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoomComfort {
    LivingRoom(RoomComfortLevel),
    Bedroom(RoomComfortLevel),
    Kitchen(RoomComfortLevel),
    RoomOfRequirements(RoomComfortLevel),
}

impl RoomComfortLevel {
    pub fn from_temperature(temperature: DegreeCelsius) -> Self {
        if temperature < DegreeCelsius(18.0) {
            Self::EnergySaving
        } else if temperature < DegreeCelsius(19.5) {
            Self::Normal
        } else {
            Self::Comfortable
        }
    }
}

impl Preconditions<HomeState> for RoomComfort {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        let (level, temperature) = match self {
            RoomComfort::LivingRoom(level) => (level, state.living_room.temperature),
            RoomComfort::Bedroom(level) => (level, state.bedroom.temperature),
            RoomComfort::Kitchen(level) => (level, state.kitchen.temperature),
            RoomComfort::RoomOfRequirements(level) => {
                (level, state.room_of_requirements.temperature)
            }
        };

        RoomComfortLevel::from_temperature(temperature) == *level
    }
}
