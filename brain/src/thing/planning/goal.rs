#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoomComfortLevel {
    EnergySaving,
    Normal,
    Comfortable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Room {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HomeGoal {
    PreventMouldInBathroom,
    StayInformed,
    AvoidUselessHeating,
    RoomComfort(Room, RoomComfortLevel),
}

//TODO select goals based on current state
pub fn get_active_goals() -> Vec<HomeGoal> {
    vec![
        HomeGoal::AvoidUselessHeating,
        HomeGoal::PreventMouldInBathroom,
        HomeGoal::StayInformed,
        HomeGoal::RoomComfort(Room::LivingRoom, RoomComfortLevel::Comfortable),
        HomeGoal::RoomComfort(Room::Bedroom, RoomComfortLevel::Normal),
        HomeGoal::RoomComfort(Room::Kitchen, RoomComfortLevel::EnergySaving),
        HomeGoal::RoomComfort(Room::RoomOfRequirements, RoomComfortLevel::EnergySaving),
        HomeGoal::RoomComfort(Room::Bathroom, RoomComfortLevel::EnergySaving),
    ]
}
