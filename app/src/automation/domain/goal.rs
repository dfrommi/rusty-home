use crate::automation::{HeatingZone, Room};
use crate::home_state::StateSnapshot;

//Refactor to variants() and is_active() method
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HomeGoal {
    PreventNoise,
    PreventMould,
    StayInformed,
    #[display("SmarterHeating[{}]", _0)]
    SmarterHeating(HeatingZone),
    #[display("BetterRoomClimate[{}]", _0)]
    BetterRoomClimate(Room),
    TvControl,
    ResetToDefaltSettings,
}

const ALL_GOALS: [HomeGoal; 12] = [
    HomeGoal::PreventNoise,
    HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
    HomeGoal::BetterRoomClimate(Room::LivingRoom),
    HomeGoal::SmarterHeating(HeatingZone::Bedroom),
    HomeGoal::BetterRoomClimate(Room::Bedroom),
    HomeGoal::SmarterHeating(HeatingZone::Kitchen),
    HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
    HomeGoal::SmarterHeating(HeatingZone::Bathroom),
    HomeGoal::PreventMould,
    HomeGoal::StayInformed,
    HomeGoal::TvControl,
    HomeGoal::ResetToDefaltSettings,
];

//TODO select goals based on current state
pub fn get_active_goals(snapshot: StateSnapshot) -> Vec<HomeGoal> {
    //TODO determine active goals
    ALL_GOALS.to_vec()
}
