use std::{collections::HashMap, sync::Arc};

use crate::{
    core::{
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    },
    home_state::{HomeState, StateValue},
    port::ValueObject,
    trigger::{UserTriggerExecution, UserTriggerTarget},
};

#[derive(Clone)]
pub struct StateSnapshot {
    timestamp: DateTime,
    data: Arc<HashMap<HomeState, DataFrame<StateValue>>>,
    active_user_triggers: Arc<HashMap<UserTriggerTarget, UserTriggerExecution>>,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        StateSnapshot {
            timestamp: DateTime::now(),
            data: Arc::new(HashMap::new()),
            active_user_triggers: Arc::new(HashMap::new()),
        }
    }
}

impl std::fmt::Debug for StateSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateSnapshot")
            .field("timestamp", &self.timestamp)
            .field("data_keys", &self.data.keys().collect::<Vec<_>>())
            .field(
                "active_user_triggers_keys",
                &self.active_user_triggers.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl StateSnapshot {
    pub fn new(
        start_of_calculation: DateTime,
        data: HashMap<HomeState, DataFrame<StateValue>>,
        active_user_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
    ) -> Self {
        StateSnapshot {
            timestamp: start_of_calculation,
            data: Arc::new(data),
            active_user_triggers: Arc::new(active_user_triggers),
        }
    }

    pub fn timestamp(&self) -> DateTime {
        self.timestamp
    }

    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject + Clone,
    {
        let state_value = self.data.get(&id.clone().into())?.last()?;
        let value = id.project_state_value(state_value.value.clone())?;

        Some(DataPoint {
            value,
            timestamp: state_value.timestamp,
        })
    }

    pub fn try_get<S>(&self, id: S) -> anyhow::Result<DataPoint<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject + Clone,
    {
        self.get(id)
            .ok_or_else(|| anyhow::anyhow!("no data point found for state"))
    }

    //Not yet needed
    // pub fn data_frame(&self, id: HomeState, range: DateTimeRange) -> Option<DataFrame<StateValue>> {
    //     self.data.get(&id)?.retain_range_with_context_before(&range)
    // }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.active_user_triggers.get(&target)
    }

    pub fn home_state_iter(&self) -> impl Iterator<Item = (&HomeState, &DataFrame<StateValue>)> {
        self.data.iter()
    }
}
