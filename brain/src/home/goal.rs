#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[allow(clippy::enum_variant_names)]
pub enum Room {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HomeGoal {
    PreventMouldInBathroom,
    StayInformed,
    #[display("SmarterHeating[{}]", _0)]
    SmarterHeating(Room),
    SaveEnergy,
    ResetToDefaltSettings,
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
        HomeGoal::SaveEnergy,
        HomeGoal::ResetToDefaltSettings,
    ]
}
