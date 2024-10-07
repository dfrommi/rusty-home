use goap::Preconditions;

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

impl Preconditions<HomeState> for RoomComfort {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        match self {
            RoomComfort::LivingRoom(_) => state.heating_output_remains_in_living_room,
            RoomComfort::Bedroom(_) => state.heating_output_remains_in_bedroom,
            RoomComfort::Kitchen(_) => state.heating_output_remains_in_kitchen,
            RoomComfort::RoomOfRequirements(_) => {
                state.heating_output_remains_in_room_of_requirements
            }
        }
    }
}
