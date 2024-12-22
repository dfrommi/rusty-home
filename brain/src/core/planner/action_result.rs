use std::{fmt::Display, sync::Mutex};

use tabled::{Table, Tabled};

use crate::port::PlanningResultTracer;

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

pub async fn display_action_results(
    action_results: &[ActionResult],
    tracer: &impl PlanningResultTracer,
) {
    if action_result_has_changed(action_results) {
        tracing::info!(
            "Planning result:\n{}",
            Table::new(action_results).to_string()
        );

        if let Err(e) = tracer.add_planning_trace(action_results).await {
            tracing::error!("Error logging planning result: {:?}", e);
        }
    } else {
        tracing::info!("Planning result is unchanged");
    }
}

impl ActionResult {
    pub fn new(action: &impl Display) -> Self {
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
fn action_result_has_changed(current: &[ActionResult]) -> bool {
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
