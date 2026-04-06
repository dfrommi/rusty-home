use crate::command::Command;
use crate::core::id::ExternalId;

use crate::trigger::UserTriggerId;

#[derive(Debug, Clone)]
pub enum ActionEvaluationResult {
    Execute(Command, ExternalId),
    ExecuteTrigger(Command, ExternalId, UserTriggerId),
    Skip,
}
