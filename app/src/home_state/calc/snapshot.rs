use std::sync::Arc;

use infrastructure::{CorrelationId, TraceContext};

use crate::{
    core::{
        time::DateTime,
        timeseries::DataPoint,
    },
    home_state::{HomeStateId, HomeStateItem, calc::StateCalculationResult},
};

#[derive(Clone, Debug)]
pub struct StateSnapshot {
    timestamp: DateTime,
    data: Arc<StateCalculationResult>,
    correlation_id: Option<CorrelationId>,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        StateSnapshot {
            timestamp: DateTime::now(),
            data: Arc::new(StateCalculationResult::default()),
            correlation_id: TraceContext::current().correlation_id(),
        }
    }
}

impl StateSnapshot {
    pub fn new(result: StateCalculationResult) -> Self {
        StateSnapshot {
            timestamp: result.timestamp(),
            data: Arc::new(result),
            correlation_id: TraceContext::current().correlation_id(),
        }
    }

    pub fn correlation_id(&self) -> Option<&CorrelationId> {
        self.correlation_id.as_ref()
    }

    pub fn timestamp(&self) -> DateTime {
        self.timestamp
    }

    #[allow(clippy::expect_used)]
    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        let state_value = self.data.get_latest(id.clone().into())?;
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
}
