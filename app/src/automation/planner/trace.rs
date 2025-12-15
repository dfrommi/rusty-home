use std::{fmt::Display, sync::Mutex};

use crate::core::time::DateTime;
use crate::t;
use infrastructure::TraceContext;

#[derive(Clone, Debug)]
pub struct PlanningTrace {
    pub timestamp: DateTime,
    pub trace_id: Option<String>,
    pub steps: Vec<PlanningTraceStep>,
}

#[derive(Clone, Debug, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlanningTraceStep {
    pub action: String,
    pub goal: String,

    pub goal_active: bool,
    pub locked: bool,
    pub fulfilled: Option<bool>,
    pub triggered: Option<bool>,

    pub correlation_id: Option<String>,
}

//Ignore correlation_id
impl PartialEq for PlanningTraceStep {
    fn eq(&self, other: &Self) -> bool {
        self.action == other.action
            && self.goal == other.goal
            && self.goal_active == other.goal_active
            && self.locked == other.locked
            && self.fulfilled == other.fulfilled
            && self.triggered == other.triggered
    }
}

impl PlanningTrace {
    pub fn new(timestamp: DateTime, trace_id: Option<String>, steps: Vec<PlanningTraceStep>) -> Self {
        Self {
            timestamp,
            trace_id,
            steps,
        }
    }

    pub fn current(steps: Vec<PlanningTraceStep>) -> Self {
        let trace_id = TraceContext::current().map(|c| c.trace_id().to_string());
        Self::new(t!(now), trace_id, steps)
    }
}

impl PlanningTraceStep {
    pub fn new(action: &impl Display, goal: &impl Display) -> Self {
        Self {
            action: format!("{action}"),
            goal: format!("{goal}"),
            goal_active: false,
            locked: false,
            fulfilled: None,
            triggered: None,
            correlation_id: None,
        }
    }
}

pub async fn display_planning_trace(trace: &PlanningTrace) {
    if planning_trace_has_changed(trace) {
        tracing::info!("Planning result:\n{:?}", trace);
    } else {
        tracing::info!("Planning result is unchanged");
    }
}

static PREVIOUS_ACTION: Mutex<Option<Vec<PlanningTraceStep>>> = Mutex::new(None);
fn planning_trace_has_changed(current: &PlanningTrace) -> bool {
    match PREVIOUS_ACTION.lock() {
        Ok(mut previous) => {
            let current = Some(current.steps.clone());
            if *previous != current {
                *previous = current;
                true
            } else {
                false
            }
        }

        Err(e) => {
            tracing::error!("Error locking previous action result, logging impacted: {:?}", e);
            false
        }
    }
}
