use std::sync::OnceLock;

use action::HomeAction;
use goal::{get_active_goals, HomeGoal};

mod action;
mod config;
mod goal;
mod planner;

use action::Action;
use tabled::Table;

#[rustfmt::skip]
fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(|| { config::default_config() })
}

pub async fn do_plan() {
    let config = default_config();
    let goals = get_active_goals();
    let action_results = planner::find_next_actions(goals, config).await;

    tracing::info!(
        "Planning result:\n{}",
        Table::new(&action_results).to_string()
    );

    for result in action_results {
        let action = result.action;
        if result.should_be_started {
            match action.start().await {
                Ok(_) => tracing::info!("Action {:?} started", action),
                Err(e) => tracing::error!("Error starting action {:?}: {:?}", action, e),
            }
        }

        if result.should_be_stopped {
            match action.stop().await {
                Ok(_) => tracing::info!("Action {:?} stopped", action),
                Err(e) => tracing::error!("Error stopping action {:?}: {:?}", action, e),
            }
        }
    }
}
