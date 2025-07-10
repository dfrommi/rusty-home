use std::sync::Arc;

use actix_web::web;

use crate::core::planner::{PlanningTrace, PlanningTraceStep};

mod overview;
mod trace;

pub fn routes(api: Arc<crate::core::HomeApi>) -> actix_web::Scope {
    web::scope("/smart_home")
        .service(overview::routes(api.clone()))
        .service(trace::routes(api))
}

struct TraceView {
    action: String,
    name: String,
    target: Option<String>,
    state: String,
}

impl From<PlanningTrace> for Vec<TraceView> {
    fn from(val: PlanningTrace) -> Self {
        val.steps.into_iter().map(Into::into).collect()
    }
}

impl From<PlanningTraceStep> for TraceView {
    fn from(val: PlanningTraceStep) -> Self {
        let (name, target) = split_action(&val.action);

        TraceView {
            action: val.action.clone(),
            name,
            target,
            state: trace_state(&val),
        }
    }
}

fn trace_state(trace: &PlanningTraceStep) -> String {
    if !trace.goal_active {
        "DISABLED"
    } else if trace.triggered == Some(true) {
        "TRIGGERED"
    } else if trace.locked {
        "LOCKED"
    } else if trace.fulfilled == Some(true) {
        "FULFILLED"
    } else {
        "UNFULFILLED"
    }
    .to_string()
}

fn split_action(input: &str) -> (String, Option<String>) {
    // Find the first '[' character
    if let Some(pos) = input.find('[') {
        let part1 = &input[..pos]; // The part before '['
        let part2 = &input[pos + 1..]; // The part after from '['

        // Remove the last ']' from the second part if it exists
        let part2 = if part2.ends_with(']') {
            &part2[..part2.len() - 1]
        } else {
            part2
        };

        (part1.to_string(), Some(part2.to_string()))
    } else {
        // If no '[' is found, return the entire string as part1 and an empty second part
        (input.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_action() {
        assert_eq!(
            split_action("FollowDefaultSetting[SetPower[Dehumidifier]]"),
            ("FollowDefaultSetting".to_string(), Some("SetPower[Dehumidifier]".to_string()))
        );
        assert_eq!(split_action("Dehumidify"), ("Dehumidify".to_string(), None));
        assert_eq!(
            split_action("IrHeaterAutoTurnOff[Bedroom]"),
            ("IrHeaterAutoTurnOff".to_string(), Some("Bedroom".to_string()))
        );
    }
}
