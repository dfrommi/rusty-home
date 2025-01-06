use std::{fmt::Display, sync::Mutex};

use crate::port::PlanningResultTracer;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanningTrace {
    pub action: String,

    pub goal: String,

    pub is_goal_active: bool,
    pub locked: bool,
    pub is_fulfilled: Option<bool>,
    pub was_triggered: Option<bool>,
}

impl PlanningTrace {
    pub fn new(action: &impl Display, goal: &impl Display) -> Self {
        Self {
            action: format!("{}", action),
            goal: format!("{}", goal),
            is_goal_active: false,
            locked: false,
            is_fulfilled: None,
            was_triggered: None,
        }
    }
}

pub async fn display_planning_trace(
    action_results: &[PlanningTrace],
    tracer: &impl PlanningResultTracer,
) {
    if planning_trace_has_changed(action_results) {
        tracing::info!("Planning result:\n{:?}", action_results);

        if let Err(e) = tracer.add_planning_trace(action_results).await {
            tracing::error!("Error logging planning result: {:?}", e);
        }
    } else {
        tracing::info!("Planning result is unchanged");
    }
}

static PREVIOUS_ACTION: Mutex<Vec<PlanningTrace>> = Mutex::new(vec![]);
fn planning_trace_has_changed(current: &[PlanningTrace]) -> bool {
    match PREVIOUS_ACTION.lock() {
        Ok(mut previous) => {
            if *previous != current {
                *previous = current.to_vec();
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
