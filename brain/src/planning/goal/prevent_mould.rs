use goap::Preconditions;

use crate::planning::HomeState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreventMould;

impl Preconditions<HomeState> for PreventMould {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        !state.risk_of_mould_in_bathroom
    }
}
