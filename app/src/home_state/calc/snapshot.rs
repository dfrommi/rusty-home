use std::sync::Arc;

use crate::{
    core::{
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    },
    home_state::{HomeStateId, HomeStateItem, HomeStateValue, calc::StateCalculationResult},
    trigger::{UserTriggerExecution, UserTriggerTarget},
};

#[derive(Clone, Debug)]
pub struct StateSnapshot {
    inner: Arc<StateCalculationResult>,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        StateSnapshot {
            inner: Arc::new(StateCalculationResult::default()),
        }
    }
}

impl StateSnapshot {
    pub fn new(result: StateCalculationResult) -> Self {
        StateSnapshot {
            inner: Arc::new(result),
        }
    }

    pub fn timestamp(&self) -> DateTime {
        self.inner.timestamp()
    }

    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        let state_value = self.inner.get_home_state_value(id.clone().into())?;
        let value = id
            .try_downcast(state_value.value.clone())
            .expect("Internal error: HomeStateValue type mismatch");

        Some(DataPoint {
            value,
            timestamp: state_value.timestamp,
        })
    }

    pub fn try_get<S>(&self, id: S) -> anyhow::Result<DataPoint<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        self.get(id)
            .ok_or_else(|| anyhow::anyhow!("no data point found for state"))
    }

    //Not yet needed
    // pub fn data_frame(&self, id: HomeState, range: DateTimeRange) -> Option<DataFrame<StateValue>> {
    //     self.data.get(&id)?.retain_range_with_context_before(&range)
    // }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.inner.user_trigger(target)
    }

    pub fn home_state_iter(&self) -> impl Iterator<Item = (&HomeStateId, &DataFrame<HomeStateValue>)> {
        self.inner.home_state_iter()
    }
}
