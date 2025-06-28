use std::sync::OnceLock;

use action::HomeAction;
use goal::HomeGoal;

mod action;
mod config;
mod goal;
pub mod state;

#[cfg(test)]
mod tests;

use crate::Database;
pub use goal::get_active_goals;
use tracing::info;

pub fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(config::default_config)
}

#[tracing::instrument(skip_all)]
pub async fn plan_for_home(api: &Database) {
    info!("Start planning");
    let active_goals = get_active_goals(api).await;
    let res = crate::core::planner::perform_planning(&active_goals, default_config(), api).await;

    match res {
        Ok(_) => info!("Planning done"),
        Err(e) => tracing::error!("Error during planning: {:?}", e),
    }
}
