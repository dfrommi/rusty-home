use std::sync::OnceLock;

use action::HomeAction;
use goal::{get_active_goals, HomeGoal};

mod action;
mod config;
mod goal;
mod planner;

#[cfg(test)]
mod tests;

use action::Action;
use planner::action_ext::ActionPlannerExt;
use tabled::Table;

use crate::{adapter::persistence::PlanLogRepository, home_api, thing::Executable};

pub use planner::ActionResult;

#[rustfmt::skip]
fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(|| { config::default_config() })
}

pub async fn do_plan() {
    let action_results = get_action_results().await;

    tracing::info!(
        "Planning result:\n{}",
        Table::new(&action_results).to_string()
    );

    if let Err(e) = home_api().add_planning_log(&action_results).await {
        tracing::error!("Error logging planning result: {:?}", e);
    }

    for result in action_results {
        let action = result.action;
        if result.should_be_started {
            match action.start_command() {
                Some(command) => match command.execute(action.command_source_start()).await {
                    Ok(_) => tracing::info!("Action {} started", action),
                    Err(e) => tracing::error!("Error starting action {}: {:?}", action, e),
                },
                None => tracing::info!(
                    "Action {} should be started, but no command is configured",
                    action
                ),
            }
        }

        if result.should_be_stopped {
            match action.stop_command() {
                Some(command) => match command.execute(action.command_source_stop()).await {
                    Ok(_) => tracing::info!("Action {} stopped", action),
                    Err(e) => tracing::error!("Error stopping action {}: {:?}", action, e),
                },
                None => tracing::info!(
                    "Action {} should be stopped, but no command is configured",
                    action
                ),
            }
        }
    }
}

async fn get_action_results() -> Vec<ActionResult<'static, HomeAction>> {
    let config = default_config();
    let goals = get_active_goals();
    planner::find_next_actions(goals, config).await
}
