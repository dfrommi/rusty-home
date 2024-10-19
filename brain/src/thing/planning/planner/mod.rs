use std::{collections::HashSet, fmt::Debug, hash::Hash};

use super::Action;

pub struct PlanningResult<'a, A> {
    pub actions_to_be_started: Vec<&'a A>,
    pub actions_to_be_stopped: Vec<&'a A>,
}

//sorting order of config is important - first come, first serve
pub async fn find_next_actions<G, A>(goals: Vec<G>, config: &[(G, Vec<A>)]) -> PlanningResult<A>
where
    G: Eq,
    A: Action + Debug,
{
    let mut actions_to_be_started: Vec<&A> = Vec::new();
    let mut actions_to_be_stopped: Vec<&A> = Vec::new();
    let mut resource_lock = ResourceLock::new();

    for (goal, actions) in config.iter() {
        let is_goal_active = goals.contains(goal);

        for action in actions {
            let used_resource = action.controls_resource();

            if resource_lock.is_locked(&used_resource) {
                continue;
            }

            let (is_fulfilled, is_user_controlled, is_running) = get_action_state(action).await;

            if is_goal_active && is_fulfilled {
                resource_lock.lock(used_resource);
            }

            if !is_user_controlled {
                if is_goal_active && is_fulfilled && !is_running {
                    actions_to_be_started.push(action);
                }

                if (!is_goal_active || !is_fulfilled) && is_running {
                    actions_to_be_stopped.push(action);
                }
            }
        }
    }

    actions_to_be_stopped.retain(|a| !resource_lock.is_locked(&a.controls_resource()));

    PlanningResult {
        actions_to_be_started,
        actions_to_be_stopped,
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
