use std::sync::OnceLock;

use action::HomeAction;
use api::command::CommandSource;
use goal::{get_active_goals, HomeGoal};

mod action;
mod config;
mod goal;
mod planner;

use action::Action;
use tabled::Table;

use crate::thing::Executable;

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
            match action.start_command() {
                Some(command) => {
                    match command
                        .execute(CommandSource::System(format!("planning:{}:start", action)))
                        .await
                    {
                        Ok(_) => tracing::info!("Action {} started", action),
                        Err(e) => tracing::error!("Error starting action {}: {:?}", action, e),
                    }
                }
                None => tracing::info!(
                    "Action {} should be started, but no command is configured",
                    action
                ),
            }
        }

        if result.should_be_stopped {
            match action.stop_command() {
                Some(command) => match command
                    .execute(CommandSource::System(format!("planning:{}:stop", action)))
                    .await
                {
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
