use goap::Preconditions;
use prevent_mould::PreventMould;
use room_comfort::RoomComfort;

use super::HomeState;

pub mod prevent_mould;
pub mod room_comfort;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HomeGoal {
    PreventMould(PreventMould),
    RoomComfort(RoomComfort),
}

impl Preconditions<HomeState> for HomeGoal {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        match self {
            HomeGoal::PreventMould(prevent_mould) => prevent_mould.is_fulfilled(state),
            HomeGoal::RoomComfort(room_comfort) => room_comfort.is_fulfilled(state),
        }
    }
}
