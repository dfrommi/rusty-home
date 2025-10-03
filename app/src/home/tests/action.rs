use crate::adapter::homekit::HomekitCommandTarget;
use crate::core::time::{DateTime, FIXED_NOW};
use crate::home::trigger::UserTriggerTarget;

use crate::{
    core::planner::{Action, ActionEvaluationResult},
    home::action::{HomeAction, UserTriggerAction},
};

use super::{infrastructure, runtime};

pub struct ActionState {
    pub is_fulfilled: bool,
}

pub fn get_state_at(iso: &str, action: impl Into<HomeAction>) -> ActionState {
    let fake_now = DateTime::from_iso(iso).unwrap();
    let action: HomeAction = action.into();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = &infrastructure().api();

        let result = action.evaluate(api).await.unwrap();

        let is_fulfilled = !matches!(result, ActionEvaluationResult::Skip);

        ActionState { is_fulfilled }
    }))
}

// #[test]
// fn user_trigger_not_started() {
//     let action = UserTriggerAction::new(UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower));
//
//     let result = get_state_at("2025-01-05T21:05:00.584641+01:00", action);
//
//     assert!(result.is_fulfilled);
// }
