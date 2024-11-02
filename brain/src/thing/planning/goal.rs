#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
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
    SmarterHeating(Room),
}

//TODO select goals based on current state
pub fn get_active_goals() -> Vec<HomeGoal> {
    vec![
        HomeGoal::SmarterHeating(Room::LivingRoom),
        HomeGoal::SmarterHeating(Room::Bedroom),
        HomeGoal::SmarterHeating(Room::Kitchen),
        HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        HomeGoal::SmarterHeating(Room::Bathroom),
        HomeGoal::PreventMouldInBathroom,
        HomeGoal::StayInformed,
    ]
}
