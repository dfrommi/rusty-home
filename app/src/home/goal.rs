use crate::core::HomeApi;
use crate::home::HeatingZone;
use crate::home::state::Powered;
use crate::port::DataPointAccess;

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
    SmarterHeating(HeatingZone),
    #[display("BetterRoomClimate[{}]", _0)]
    BetterRoomClimate(Room),
    TvControl,
    CoreControl,
    ResetToDefaltSettings,
}

//TODO select goals based on current state
pub async fn get_active_goals(api: &HomeApi) -> Vec<HomeGoal> {
    //TODO auto-detect summer mode
    let mut goals = vec![
        HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        HomeGoal::SmarterHeating(HeatingZone::Bedroom),
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        HomeGoal::SmarterHeating(HeatingZone::Kitchen),
        HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
        //HomeGoal::SmarterHeating(Room::Bathroom),
        HomeGoal::PreventMouldInBathroom,
        HomeGoal::StayInformed,
        HomeGoal::TvControl,
        HomeGoal::CoreControl,
        HomeGoal::ResetToDefaltSettings,
    ];

    if Powered::LivingRoomTv.current(api).await.unwrap_or(false) {
        goals.push(HomeGoal::TvControl);
    }

    goals
}
