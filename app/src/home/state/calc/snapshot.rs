use std::{collections::HashMap, sync::Arc};

use crate::{
    core::{
        persistence::UserTriggerRequest,
        time::DateTimeRange,
        timeseries::{DataFrame, DataPoint},
    },
    home::{
        state::{HomeState, StateValue},
        trigger::UserTriggerTarget,
    },
    port::ValueObject,
};

#[derive(Clone)]
pub struct StateSnapshot {
    data: Arc<HashMap<HomeState, DataFrame<StateValue>>>,
    active_user_triggers: Arc<HashMap<UserTriggerTarget, UserTriggerRequest>>,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        StateSnapshot {
            data: Arc::new(HashMap::new()),
            active_user_triggers: Arc::new(HashMap::new()),
        }
    }
}

impl StateSnapshot {
    pub fn new(
        data: HashMap<HomeState, DataFrame<StateValue>>,
        active_user_triggers: HashMap<UserTriggerTarget, UserTriggerRequest>,
    ) -> Self {
        StateSnapshot {
            data: Arc::new(data),
            active_user_triggers: Arc::new(active_user_triggers),
        }
    }

    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject,
    {
        let id = id.into();
        let state_value = self.data.get(&id)?.last();
        let value = S::project_state_value(state_value.value.clone())?;

        Some(DataPoint {
            value,
            timestamp: state_value.timestamp,
        })
    }

    pub fn data_frame(&self, id: HomeState, range: DateTimeRange) -> Option<DataFrame<StateValue>> {
        self.data.get(&id)?.retain_range_with_context_before(&range)
    }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerRequest> {
        self.active_user_triggers.get(&target)
    }

    pub(super) fn home_state_iter(&self) -> impl Iterator<Item = (&HomeState, &DataFrame<StateValue>)> {
        self.data.iter()
    }
}
