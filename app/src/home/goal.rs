use crate::home_state::{IsRunning, StateSnapshot};
use crate::home::{HeatingZone, Room};

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HomeGoal {
    PreventMouldInBathroom,
    StayInformed,
    #[display("SmarterHeating[{}]", _0)]
    SmarterHeating(HeatingZone),
    #[display("BetterRoomClimate[{}]", _0)]
    BetterRoomClimate(Room),
    TvControl,
    CoreControl,
    ResetToDefaltSettings,
}

//TODO select goals based on current state
pub fn get_active_goals(snapshot: StateSnapshot) -> Vec<HomeGoal> {
    //TODO auto-detect summer mode
    let mut goals = vec![
        HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        HomeGoal::SmarterHeating(HeatingZone::Bedroom),
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        HomeGoal::SmarterHeating(HeatingZone::Kitchen),
        HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
        HomeGoal::SmarterHeating(HeatingZone::Bathroom),
        HomeGoal::PreventMouldInBathroom,
        HomeGoal::StayInformed,
        HomeGoal::TvControl,
        HomeGoal::CoreControl,
        HomeGoal::ResetToDefaltSettings,
    ];

    if snapshot
        .get(IsRunning::LivingRoomTv)
        .map(|dp| dp.value)
        .unwrap_or(false)
    {
        goals.push(HomeGoal::TvControl);
    }

    goals
}
