mod action_execution_state;
mod command_state;
mod resource_lock;

use std::{
    fmt::{Debug, Display},
    sync::Mutex,
};

pub use action_execution_state::ActionExecutionState;
use api::command::Command;
pub use command_state::CommandState;
use resource_lock::ResourceLock;
use support::t;
use tabled::{Table, Tabled};

use crate::port::{CommandAccess, CommandExecutor};

use super::{action::Action, PlanningResultTracer};

#[derive(Clone, Debug, PartialEq, Eq, Tabled)]
pub struct ActionResult {
    pub action: String,
    #[tabled(display_with = "display_bool")]
    pub should_be_started: bool,
    #[tabled(display_with = "display_bool")]
    pub should_be_stopped: bool,
    #[tabled(display_with = "display_bool")]
    pub is_goal_active: bool,
    #[tabled(display_with = "display_bool")]
    pub locked: bool,
    #[tabled(display_with = "display_option")]
    pub is_fulfilled: Option<bool>,
    #[tabled(display_with = "display_option")]
    pub is_running: Option<bool>,
}

pub async fn do_plan<G, A, T>(active_goals: &[G], config: &[(G, Vec<A>)], api: &T)
where
    G: Eq,
    A: Action<T>,
    T: PlanningResultTracer + CommandAccess<Command> + CommandExecutor<Command>,
    Command: CommandState<T>,
{
    let next_actions = find_next_actions(active_goals, config, api).await;
    let action_results = next_actions.iter().map(|(_, r)| r).collect::<Vec<_>>();

    if action_result_has_changed(&action_results) {
        tracing::info!(
            "Planning result:\n{}",
            Table::new(&action_results).to_string()
        );

        if let Err(e) = api.add_planning_trace(&action_results).await {
            tracing::error!("Error logging planning result: {:?}", e);
        }
    } else {
        tracing::info!("Planning result is unchanged");
    }

    for (action, result) in next_actions {
        if result.should_be_started {
            match action.start_command() {
                Some(command) => match api.execute(command, action.start_command_source()).await {
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
                Some(command) => match api.execute(command, action.stop_command_source()).await {
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

//sorting order of config is important - first come, first serve
pub async fn find_next_actions<'a, G, A, T>(
    goals: &'a [G],
    config: &'a [(G, Vec<A>)],
    api: &T,
) -> Vec<(&'a A, ActionResult)>
where
    G: Eq,
    A: Action<T>,
    T: CommandAccess<Command>,
    Command: CommandState<T>,
{
    let mut resource_lock = ResourceLock::new();
    let mut action_results: Vec<(&'a A, ActionResult)> = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = goals.contains(goal);

        for action in actions {
            let mut result = ActionResult::new(action);
            result.is_goal_active = is_goal_active;

            let used_resource = action.controls_target();

            if resource_lock.is_locked(&used_resource) {
                result.locked = true;
                action_results.push((action, result));
                continue;
            }

            let (is_fulfilled, is_running) = tokio::join!(
                is_fulfilled_or_just_triggered(action, api),
                is_running_or_just_triggered(action, api),
            );

            let is_fulfilled = is_fulfilled.unwrap_or_else(|e| {
                tracing::warn!(
                    "Error checking preconditions of action {}, assuming not fulfilled: {:?}",
                    action,
                    e
                );
                false
            });

            let is_running = is_running.unwrap_or_else(|e| {
                tracing::warn!(
                    "Error checking running state of action {}, assuming not running: {:?}",
                    action,
                    e
                );
                None
            });

            result.is_fulfilled = Some(is_fulfilled);
            result.is_running = is_running;

            if is_goal_active && is_fulfilled {
                resource_lock.lock(used_resource);
                result.should_be_started = is_running == Some(false);
            }

            if !is_goal_active || !is_fulfilled {
                let has_stop_action = action.stop_command().is_some();
                result.should_be_stopped = (is_running == Some(true)) && has_stop_action;
            }

            action_results.push((action, result));
        }
    }

    for (action, result) in action_results.iter_mut() {
        if result.should_be_stopped {
            let resource = action.controls_target();

            if resource_lock.is_locked(&resource) {
                result.should_be_stopped = false;
                result.locked = true;
            } else {
                resource_lock.lock(resource);
            }
        }
    }

    action_results
}

async fn is_fulfilled_or_just_triggered<A, T>(action: &A, api: &T) -> anyhow::Result<bool>
where
    A: Action<T>,
    T: CommandAccess<Command>,
{
    if action
        .start_latest_trigger_since(api, t!(30 seconds ago))
        .await?
    {
        return Ok(true);
    } else if action
        .stop_latest_trigger_since(api, t!(30 seconds ago))
        .await?
    {
        return Ok(false);
    }

    action.preconditions_fulfilled(api).await
}

async fn is_running_or_just_triggered<A, T>(action: &A, api: &T) -> anyhow::Result<Option<bool>>
where
    A: ActionExecutionState<T>,
    T: CommandAccess<Command>,
    Command: CommandState<T>,
{
    if action
        .start_latest_trigger_since(api, t!(30 seconds ago))
        .await?
    {
        return Ok(Some(true));
    } else if action
        .stop_latest_trigger_since(api, t!(30 seconds ago))
        .await?
    {
        return Ok(Some(false));
    }

    action.is_running(api).await
}

impl ActionResult {
    fn new(action: &impl Display) -> Self {
        Self {
            action: format!("{}", action),
            should_be_started: false,
            should_be_stopped: false,
            is_goal_active: false,
            locked: false,
            is_fulfilled: None,
            is_running: None,
        }
    }
}

fn display_bool(b: &bool) -> String {
    display_option(&Some(*b))
}

fn display_option(o: &Option<bool>) -> String {
    match o {
        Some(true) => "✅".to_string(),
        Some(false) => "❌".to_string(),
        None => "-".to_string(),
    }
}

static PREVIOUS_ACTION: Mutex<Vec<ActionResult>> = Mutex::new(vec![]);
fn action_result_has_changed(current: &[&ActionResult]) -> bool {
    match PREVIOUS_ACTION.lock() {
        Ok(mut previous) => {
            let previous_refs: Vec<&ActionResult> = previous.iter().collect();

            if previous_refs != current {
                *previous = current.iter().map(|&r| r.clone()).collect();
                true
            } else {
                false
            }
        }

        Err(e) => {
            tracing::error!(
                "Error locking previous action result, logging impacted: {:?}",
                e
            );
            false
        }
    }
}
