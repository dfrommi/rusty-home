use std::{cell::RefCell, collections::HashMap};

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateClient, DeviceStateId, DeviceStateItem, DeviceStateValue},
    home_state::{HomeStateDerivedStateProvider, HomeStateId, HomeStateItem, HomeStateValue},
    t,
    trigger::{TriggerClient, UserTriggerExecution, UserTriggerTarget},
};

use super::StateSnapshot;

pub async fn create_standalone_context(
    device_state: &DeviceStateClient,
    trigger_client: &TriggerClient,
) -> anyhow::Result<StateCalculationContext> {
    let start_time = t!(now);

    let triggers = trigger_client.get_all_active_triggers().await?;
    let mut active_triggers = HashMap::new();

    for trigger in triggers {
        active_triggers.insert(trigger.target(), trigger);
    }

    let device_states = device_state.get_current_for_all().await?;

    Ok(StateCalculationContext {
        start_time,
        current: RefCell::new(HashMap::new()),
        device_state: device_states,
        active_user_triggers: active_triggers,
        prev: None,
    })
}

pub struct StateCalculationContext {
    pub start_time: DateTime,
    current: RefCell<HashMap<HomeStateId, DataPoint<HomeStateValue>>>,
    device_state: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>,
    active_user_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
    pub prev: Option<Box<StateCalculationContext>>,
}

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<T>;
}

impl StateCalculationContext {
    pub fn with_history(
        self,
        mut previous: Option<StateCalculationContext>,
        keep: Duration,
    ) -> StateCalculationContext {
        let cutoff = self.start_time - keep;
        if let Some(prev_ctx) = previous.as_mut() {
            prev_ctx.truncate_before(cutoff);
        }
        StateCalculationContext {
            start_time: self.start_time,
            current: self.current,
            device_state: self.device_state,
            active_user_triggers: self.active_user_triggers,
            prev: previous.map(Box::new),
        }
    }

    pub fn range(&self) -> DateTimeRange {
        let mut start = self.start_time;
        let mut current_ctx = self;

        while let Some(prev) = &current_ctx.prev {
            start = prev.start_time;
            current_ctx = prev;
        }

        DateTimeRange::new(start, self.start_time)
    }

    pub fn load_all(&self) {
        for id in HomeStateId::variants().iter() {
            self.get_home_state_value(*id);
        }
    }

    fn truncate_before(&mut self, timestamp: DateTime) {
        let mut current_ctx = self;

        loop {
            let should_cut = current_ctx
                .prev
                .as_ref()
                .is_some_and(|prev| prev.start_time < timestamp);

            if should_cut {
                if let Some(ref prev) = current_ctx.prev {
                    tracing::debug!(
                        "Truncating state calculation context history at timestamp {}, cutting context starting at {}",
                        timestamp,
                        prev.start_time
                    );
                }

                current_ctx.prev = None;
                break;
            }

            match current_ctx.prev.as_mut() {
                Some(prev) => current_ctx = prev,
                None => break,
            }
        }
    }

    pub fn as_snapshot(&self) -> StateSnapshot {
        let mut data = HashMap::new();

        for id in self.current.borrow().keys() {
            match self.data_frame(*id, DateTime::min_value()) {
                Some(df) => {
                    data.insert(*id, df);
                }
                None => {
                    tracing::warn!("No data-frame found, but current value exists for state {:?}", id);
                    continue;
                }
            }
        }

        StateSnapshot::new(self.start_time, data, self.active_user_triggers.clone())
    }
}

impl StateCalculationContext {
    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        let state_value = self.get_home_state_value(id.clone().into())?;
        match id.try_downcast(state_value.value.clone()) {
            Ok(v) => Some(state_value.with(v)),
            Err(e) => {
                tracing::error!("Error converting home state {:?} to exepceted type: {}", state_value.value, e);
                None
            }
        }
    }

    pub fn all_since<S>(&self, id: S, since: DateTime) -> Option<DataFrame<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        let df = self
            .data_frame(id.clone().into(), since)?
            .map(|dp: &DataPoint<HomeStateValue>| {
                id.try_downcast(dp.value.clone())
                    .expect("Internal error: State value projection failed in all_since")
            });
        Some(df)
    }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.active_user_triggers.get(&target)
    }

    pub fn device_state<D>(&self, id: D) -> Option<DataPoint<D::Type>>
    where
        D: Into<DeviceStateId> + DeviceStateItem + Clone,
    {
        let dp = self.device_state.get(&id.clone().into())?;
        match id.try_downcast(dp.value.clone()) {
            Ok(v) => Some(DataPoint::new(v, dp.timestamp)),
            Err(e) => {
                tracing::error!("Error converting device state {:?} to exepceted type: {}", dp.value, e);
                None
            }
        }
    }
}

impl StateCalculationContext {
    //TODO try to use ref
    fn get_home_state_value(&self, id: HomeStateId) -> Option<DataPoint<HomeStateValue>> {
        let current_value = {
            let current = self.current.borrow();
            current.get(&id).cloned()
        };

        match current_value {
            Some(dp) => Some(dp),
            None => {
                let calculated_value = HomeStateDerivedStateProvider.calculate_current(id, self)?;
                let previous_dp = self.prev.as_ref().and_then(|ctx| ctx.get_home_state_value(id));

                //check if previous value is the same, then reuse timestamp
                let calculated_dp = if let Some(previous_dp) = previous_dp
                    && previous_dp.value == calculated_value
                {
                    previous_dp.clone()
                } else {
                    //no or different previous value
                    DataPoint::new(calculated_value, self.start_time)
                };

                let mut current_mut = self.current.borrow_mut();
                current_mut.insert(id, calculated_dp.clone());
                Some(calculated_dp)
            }
        }
    }

    fn data_frame(&self, id: HomeStateId, since: DateTime) -> Option<DataFrame<HomeStateValue>> {
        let mut current_ctx = self;
        let mut dps = vec![];

        while current_ctx.start_time >= since {
            if let Some(dp) = current_ctx.get_home_state_value(id) {
                dps.push(dp);
            }

            match &current_ctx.prev {
                Some(prev) => current_ctx = prev,
                None => break,
            }
        }

        if dps.is_empty() {
            None
        } else {
            Some(DataFrame::new(dps))
        }
    }
}

fn downcast_df<S: HomeStateItem + Into<HomeStateId>>(
    id: S,
    df: &DataFrame<HomeStateValue>,
) -> Option<DataFrame<S::Type>> {
    Some(df.map(|dp: &DataPoint<HomeStateValue>| {
        //TODO fail gracefully
        id.try_downcast(dp.value.clone())
            .expect("Internal error: State value projection failed in downcast_df")
    }))
}
