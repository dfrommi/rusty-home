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
use tracing::info;

pub fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(|| config::default_config())
}

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(api: &super::Database) {
    info!("Start planning");
    let active_goals = get_active_goals(api).await;
    crate::core::planner::perform_planning(&active_goals, default_config(), api, api, api).await;
    info!("Planning done");
}
