mod processor;

use crate::command::Command;
use crate::core::id::ExternalId;

use crate::trigger::UserTriggerId;

pub use processor::plan_for_home;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    ExecuteDefault,
    Execute(Command, ExternalId),
    ExecuteTrigger(Command, ExternalId, UserTriggerId),
    Skip,
}
