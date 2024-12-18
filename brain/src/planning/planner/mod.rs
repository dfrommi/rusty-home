mod action_execution;
mod command_state;
mod resource_lock;

use std::{
    fmt::{Debug, Display},
    sync::Mutex,
};

pub use action_execution::{ActionExecution, ActionExecutionTrigger};
use api::command::{Command, CommandTarget};
pub use command_state::CommandState;
use resource_lock::ResourceLock;
use support::t;
use tabled::{Table, Tabled};

use crate::port::{CommandAccess, CommandExecutor, PlanningResultTracer};

use super::action::Action;

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

pub async fn plan_and_execute<G, A, T>(
    active_goals: &[G],
    config: &[(G, Vec<A>)],
    api: &T,
    result_tracer: &impl PlanningResultTracer,
    command_processor: &(impl CommandAccess<Command> + CommandExecutor<Command> + CommandState<Command>),
) where
    G: Eq,
    A: Action<T>,
{
    let next_actions = find_next_actions(active_goals, config, api, command_processor).await;
    let action_results = next_actions.iter().map(|(_, r)| r).collect::<Vec<_>>();

    if action_result_has_changed(&action_results) {
        tracing::info!(
            "Planning result:\n{}",
            Table::new(&action_results).to_string()
        );

        if let Err(e) = result_tracer.add_planning_trace(&action_results).await {
            tracing::error!("Error logging planning result: {:?}", e);
        }
    } else {
        tracing::info!("Planning result is unchanged");
    }

    for (action, result) in next_actions {
        if result.should_be_started {
            if let Err(e) = action.execution().execute_start(command_processor).await {
                tracing::error!("Error starting action {}: {:?}", action, e);
            }
        }

        if result.should_be_stopped {
            if let Err(e) = action.execution().execute_stop(command_processor).await {
                tracing::error!("Error stopping action {}: {:?}", action, e);
            }
        }
    }
}

//sorting order of config is important - first come, first serve
pub async fn find_next_actions<'a, G, A, T>(
    goals: &'a [G],
    config: &'a [(G, Vec<A>)],
    api: &T,
    command_processor: &(impl CommandAccess<Command> + CommandState<Command>),
) -> Vec<(&'a A, ActionResult)>
where
    G: Eq,
    A: Action<T>,
{
    let mut resource_lock: ResourceLock<CommandTarget> = ResourceLock::new();
    let mut action_results: Vec<(&'a A, ActionResult)> = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = goals.contains(goal);

        for action in actions {
            let mut result = ActionResult::new(action);
            result.is_goal_active = is_goal_active;

            let action_execution = action.execution();

            let used_resource = action_execution.controlled_target();

            if resource_lock.is_locked(used_resource) {
                result.locked = true;
                action_results.push((action, result));
                continue;
            }

            let (is_fulfilled, is_running) =
                get_fulfilled_and_running_state(action, api, command_processor).await;

            result.is_fulfilled = Some(is_fulfilled);
            result.is_running = is_running;

            if is_goal_active && is_fulfilled {
                resource_lock.lock(used_resource.clone());
                result.should_be_started =
                    action_execution.can_be_started() && is_running == Some(false);
            }

            if !is_goal_active || !is_fulfilled {
                result.should_be_stopped =
                    action_execution.can_be_stopped() && (is_running == Some(true));
            }

            action_results.push((action, result));
        }
    }

    for (action, result) in action_results.iter_mut() {
        if result.should_be_stopped {
            let resource = action.execution().controlled_target();

            if resource_lock.is_locked(resource) {
                result.should_be_stopped = false;
                result.locked = true;
            } else {
                resource_lock.lock(resource.clone());
            }
        }
    }

    action_results
}

async fn get_fulfilled_and_running_state<A, T>(
    action: &A,
    api: &T,
    command_access: &(impl CommandAccess<Command> + CommandState<Command>),
) -> (bool, Option<bool>)
where
    A: Action<T>,
{
    macro_rules! unwrap_or_warn {
        ($e:expr, $default:expr, $msg:literal) => {
            $e.unwrap_or_else(|e| {
                tracing::warn!($msg, action, e);
                $default
            })
        };
    }

    let action_execution = action.execution();

    let latest_trigger = unwrap_or_warn!(
        action_execution
            .latest_trigger_since(command_access, t!(30 seconds ago))
            .await,
        ActionExecutionTrigger::None,
        "Error getting latest exexcution of action {}, assuming not running: {:?}"
    );

    if latest_trigger == ActionExecutionTrigger::Start {
        return (true, Some(true));
    } else if latest_trigger == ActionExecutionTrigger::Stop {
        return (false, Some(false));
    }

    let (action_preconditions_fulfilled, execution_started_and_still_reflected) = tokio::join!(
        action.preconditions_fulfilled(api),
        action_execution.action_started_and_still_reflected(command_access),
    );

    let action_preconditions_fulfilled = unwrap_or_warn!(
        action_preconditions_fulfilled,
        false,
        "Error checking preconditions of action {}, assuming not fulfilled: {:?}"
    );

    let execution_started_and_still_reflected = unwrap_or_warn!(
        execution_started_and_still_reflected,
        None,
        "Error checking running state of action {}, assuming not running: {:?}"
    );

    (
        action_preconditions_fulfilled,
        execution_started_and_still_reflected,
    )
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
