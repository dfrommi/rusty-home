use api::state::Powered;

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
    SmarterHeating(Room),
    #[display("BetterRoomClimate[{}]", _0)]
    BetterRoomClimate(Room),
    TvControl,
    ResetToDefaltSettings,
}

//TODO select goals based on current state
pub async fn get_active_goals<API>(api: &API) -> Vec<HomeGoal>
where
    API: DataPointAccess<Powered>,
{
    //TODO auto-detect summer mode
    let mut goals = vec![
        //HomeGoal::SmarterHeating(Room::LivingRoom),
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        //HomeGoal::SmarterHeating(Room::Bedroom),
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        //HomeGoal::SmarterHeating(Room::Kitchen),
        //HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        //HomeGoal::SmarterHeating(Room::Bathroom),
        HomeGoal::PreventMouldInBathroom,
        //HomeGoal::StayInformed,
        HomeGoal::TvControl,
        HomeGoal::ResetToDefaltSettings,
    ];

    if api.current(Powered::LivingRoomTv).await.unwrap_or(false) {
        goals.push(HomeGoal::TvControl);
    }

    goals
}
