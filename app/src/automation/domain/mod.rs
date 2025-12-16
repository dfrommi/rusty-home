use std::sync::OnceLock;

use action::HomeAction;
use goal::HomeGoal;

mod action;
mod config;
mod goal;

mod common;

pub use action::RuleEvaluationContext;
pub use common::*;
pub use goal::get_active_goals;

use crate::home_state::StateSnapshot;

pub struct HomePlanning;

impl HomePlanning {
    pub fn active_goals(snapshot: StateSnapshot) -> Vec<HomeGoal> {
        get_active_goals(snapshot)
    }

    pub fn config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
        static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
        CONFIG.get_or_init(config::default_config)
    }
}
