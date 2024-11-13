pub mod action_ext;
mod resource_lock;

use std::fmt::{Debug, Display};

use resource_lock::ResourceLock;
use tabled::Tabled;

use super::Action;
use action_ext::ActionPlannerExt;

#[derive(Debug, Tabled)]
pub struct ActionResult<'a, A: Display> {
    pub action: &'a A,
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

//sorting order of config is important - first come, first serve
pub async fn find_next_actions<G, A>(goals: Vec<G>, config: &[(G, Vec<A>)]) -> Vec<ActionResult<A>>
where
    G: Eq,
    A: Action + Debug,
{
    let mut resource_lock = ResourceLock::new();
    let mut action_results: Vec<ActionResult<A>> = Vec::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = goals.contains(goal);

        for action in actions {
            let mut result = ActionResult::new(action);
            result.is_goal_active = is_goal_active;

            let used_resource = action.controls_target();

            if resource_lock.is_locked(&used_resource) {
                result.locked = true;
                action_results.push(result);
                continue;
            }

            let (is_fulfilled, is_running) = if action.just_started().await {
                (true, true)
            } else if action.just_stopped().await {
                (false, false)
            } else {
                tokio::join!(
                    action.preconditions_fulfilled_or_default(),
                    action.is_running_or_scheduled_or_default(),
                )
            };

            tokio::join!(
                action.preconditions_fulfilled_or_default(),
                action.is_running_or_scheduled_or_default(),
            );

            result.is_fulfilled = Some(is_fulfilled);
            result.is_running = Some(is_running);

            if is_goal_active && is_fulfilled {
                resource_lock.lock(used_resource);
                result.should_be_started = !is_running;
            }

            if !is_goal_active || !is_fulfilled {
                result.should_be_stopped = is_running;
            }

            action_results.push(result);
        }
    }

    for result in action_results.iter_mut() {
        if result.should_be_stopped {
            let resource = result.action.controls_target();

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

impl<'a, A: Action> ActionResult<'a, A> {
    fn new(action: &'a A) -> Self {
        Self {
            action,
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
