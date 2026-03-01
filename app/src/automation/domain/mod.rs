use goal::HomeGoal;

mod action;
mod goal;

pub use action::RuleEvaluationContext;
pub use goal::get_active_goals;

use crate::home_state::StateSnapshot;

pub struct HomePlanning;

impl HomePlanning {
    pub fn active_goals(snapshot: StateSnapshot) -> Vec<HomeGoal> {
        get_active_goals(snapshot)
    }
}
