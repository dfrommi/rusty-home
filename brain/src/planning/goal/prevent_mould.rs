use goap::Preconditions;

use crate::planning::HomeState;

pub struct PreventMouldGoal;

impl Preconditions<HomeState> for PreventMouldGoal {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        !state.risk_of_mould_in_bathroom
    }
}
