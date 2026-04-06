use std::{fmt::Display, sync::Mutex};

use infrastructure::CorrelationId;

#[derive(Clone, Debug)]
pub struct PlanningTrace {
    pub steps: Vec<PlanningTraceStep>,
}

#[derive(Clone, Debug, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlanningTraceStep {
    pub action: String,
    pub resource: String,

    pub fulfilled: Option<bool>,
    pub triggered: Option<bool>,

    pub correlation_id: Option<CorrelationId>,
}

impl PartialEq for PlanningTraceStep {
    fn eq(&self, other: &Self) -> bool {
        self.action == other.action
            && self.resource == other.resource
            && self.fulfilled == other.fulfilled
            && self.triggered == other.triggered
    }
}

impl PlanningTrace {
    pub fn new(steps: Vec<PlanningTraceStep>) -> Self {
        Self { steps }
    }
}

impl PlanningTraceStep {
    pub fn new(action: &impl Display, resource: &impl Display) -> Self {
        Self {
            action: format!("{action}"),
            resource: format!("{resource}"),
            fulfilled: None,
            triggered: None,
            correlation_id: None,
        }
    }
}

pub fn display_planning_trace(trace: &PlanningTrace) {
    if planning_trace_has_changed(trace) {
        tracing::info!("Planning result changed");
    } else {
        tracing::debug!("Planning result is unchanged");
    }
}

static PREVIOUS_TRACE: Mutex<Option<Vec<PlanningTraceStep>>> = Mutex::new(None);
fn planning_trace_has_changed(current: &PlanningTrace) -> bool {
    match PREVIOUS_TRACE.lock() {
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
            tracing::error!("Error locking previous trace, logging impacted: {:?}", e);
            false
        }
    }
}
