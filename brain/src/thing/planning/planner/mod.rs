use std::{collections::HashSet, fmt::Debug, hash::Hash};

use tabled::Tabled;

use super::Action;

#[derive(Debug, Tabled)]
pub struct ActionResult<'a, A: Action> {
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
    #[tabled(display_with = "display_option")]
    pub is_user_controlled: Option<bool>,
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

            let used_resource = action.controls_resource();

            if resource_lock.is_locked(&used_resource) {
                result.locked = true;
                action_results.push(result);
                continue;
            }

            let (is_fulfilled, is_user_controlled, is_running) = get_action_state(action).await;
            result.is_fulfilled = Some(is_fulfilled);
            result.is_user_controlled = Some(is_user_controlled);
            result.is_running = Some(is_running);

            if is_goal_active && is_fulfilled {
                resource_lock.lock(used_resource);
            }

            if !is_user_controlled {
                if is_goal_active && is_fulfilled && !is_running {
                    result.should_be_started = true;
                }

                if (!is_goal_active || !is_fulfilled) && is_running {
                    result.should_be_stopped = true;
                }
            }

            action_results.push(result);
        }
    }

    for result in action_results.iter_mut() {
        if result.should_be_stopped && resource_lock.is_locked(&result.action.controls_resource()) {
            result.should_be_stopped = false;
            result.locked = true;
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
            is_user_controlled: None,
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

async fn get_action_state(action: &impl Action) -> (bool, bool, bool) {
    let (is_fulfilled_res, is_user_controlled_res, is_running_res) = tokio::join!(
        action.preconditions_fulfilled(),
        action.is_user_controlled(),
        action.is_running(),
    );

    let is_fulfilled = is_fulfilled_res.unwrap_or_else(|e| {
        tracing::warn!(
            "Error checking preconditions of action {:?}, assuming not fulfilled: {:?}",
            action,
            e
        );
        false
    });

    let is_user_controlled = is_user_controlled_res.unwrap_or_else(|e| {
        tracing::warn!(
            "Error checking user-controlled state of action {:?}, assuming not controlled: {:?}",
            action,
            e
        );
        false
    });

    let is_running = is_running_res.unwrap_or_else(|e| {
        tracing::warn!(
            "Error checking running state of action {:?}, assuming not running: {:?}",
            action,
            e
        );
        false
    });

    (is_fulfilled, is_user_controlled, is_running)
}

struct ResourceLock<R> {
    resources: HashSet<R>,
}

impl<R> ResourceLock<R>
where
    R: Eq + Hash,
{
    fn new() -> Self {
        Self {
            resources: HashSet::new(),
        }
    }

    fn lock(&mut self, resource: Option<R>) {
        if let Some(resource) = resource {
            self.resources.insert(resource);
        }
    }

    fn is_locked(&self, resource: &Option<R>) -> bool {
        resource
            .as_ref()
            .map_or(false, |resource| self.resources.contains(resource))
    }
}
