use std::{cell::RefCell, collections::HashMap};

use crate::{
    core::{
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateId, DeviceStateItem, DeviceStateValue},
    home_state::{HomeStateDerivedStateProvider, HomeStateId, HomeStateItem, HomeStateValue},
    t,
    trigger::{UserTriggerExecution, UserTriggerTarget},
};

use super::StateSnapshot;

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<T>;
}

pub trait DeviceStateProvider: Send + 'static {
    fn get(&self, id: &DeviceStateId) -> Option<DataPoint<DeviceStateValue>>;
}

pub trait UserTriggerProvider: Send + 'static {
    fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution>;
    fn get_all(&self) -> HashMap<UserTriggerTarget, UserTriggerExecution>;
}

pub struct StateCalculationContext {
    start_time: DateTime,
    current: RefCell<HashMap<HomeStateId, DataPoint<HomeStateValue>>>,
    device_state: Box<dyn DeviceStateProvider>,
    active_user_triggers: Box<dyn UserTriggerProvider>,
    prev: Option<Box<StateCalculationContext>>,
}

impl StateCalculationContext {
    pub fn new<D: DeviceStateProvider, T: UserTriggerProvider>(
        device_state: D,
        active_user_triggers: T,
        mut previous: Option<StateCalculationContext>,
        keep: Duration,
    ) -> Self {
        let start_time = t!(now);
        let cutoff = start_time - keep;
        if let Some(prev_ctx) = previous.as_mut() {
            prev_ctx.truncate_before(cutoff);
        }

        StateCalculationContext {
            start_time,
            current: RefCell::new(HashMap::new()),
            device_state: Box::new(device_state),
            active_user_triggers: Box::new(active_user_triggers),
            prev: previous.map(Box::new),
        }
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

        StateSnapshot::new(self.start_time, data, self.active_user_triggers.get_all())
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

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<UserTriggerExecution> {
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
