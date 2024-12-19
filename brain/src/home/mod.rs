use std::sync::OnceLock;

use action::HomeAction;
use goal::HomeGoal;

mod action;
mod config;
mod goal;
pub mod state;

#[cfg(test)]
mod tests;

pub use goal::get_active_goals;

pub fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(|| config::default_config())
}
