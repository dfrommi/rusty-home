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

pub async fn calculate_new_snapshot(
    truncate_before: Duration,
    history: &StateSnapshot,
    device_state: &DeviceStateClient,
    trigger_client: &TriggerClient,
) -> anyhow::Result<StateSnapshot> {
    let context = create_new_calculation_context(history, device_state, trigger_client).await?;

    //preload all current values
    for id in HomeStateId::variants().iter() {
        context.load(*id);
    }

    let snapshot = context.into_snapshot(DateTimeRange::since(t!(now) - truncate_before));

    Ok(snapshot)
}

//TODO optimize creation of context using less copy and loops, ideally Arc
async fn create_new_calculation_context(
    history: &StateSnapshot,
    device_state: &DeviceStateClient,
    trigger_client: &TriggerClient,
) -> anyhow::Result<StateCalculationContext> {
    let start_time = t!(now);

    let triggers = trigger_client.get_all_active_triggers().await?;
    let mut active_triggers = HashMap::new();

    for trigger in triggers {
        active_triggers.insert(trigger.target(), trigger);
    }

    let mut history_map: HashMap<HomeStateId, DataFrame<HomeStateValue>> = HashMap::new();
    for (id, value) in history.home_state_iter() {
        history_map.insert(id.clone(), value.clone());
    }

    let device_states = device_state.get_current_for_all().await?;

    Ok(StateCalculationContext {
        start_time,
        current: RefCell::new(HashMap::new()),
        history: history_map,
        device_state: device_states,
        active_user_triggers: active_triggers,
    })
}

pub struct StateCalculationContext {
    start_time: DateTime,
    current: RefCell<HashMap<HomeStateId, DataPoint<HomeStateValue>>>,
    history: HashMap<HomeStateId, DataFrame<HomeStateValue>>,
    device_state: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>,
    active_user_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
}

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<T>;
}

impl StateCalculationContext {
    fn load(&self, id: HomeStateId) {
        let _ = self.get_home_state_value(id);
    }

    fn into_snapshot(self, range: DateTimeRange) -> StateSnapshot {
        let mut data = HashMap::new();

        for (id, current) in self.current.borrow().iter() {
            let df: Option<DataFrame<HomeStateValue>> = self.history.get(id).cloned();
            let combined_df = match df {
                Some(mut df) => {
                    df.insert(current.clone());
                    df.retain_range_with_context_before(&range)
                }
                None => DataFrame::new(vec![current.clone()]),
            };
            data.insert(*id, combined_df);
        }

        StateSnapshot::new(self.start_time, data, self.active_user_triggers)
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

    pub fn all_since<S>(&self, id: S, timestamp: DateTime) -> Option<DataFrame<S::Type>>
    where
        S: Into<HomeStateId> + HomeStateItem + Clone,
    {
        let df = self.data_frame(id.clone().into(), DateTimeRange::since(timestamp))?;
        downcast_df(id, &df)
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
    fn get_home_state_value(&self, id: HomeStateId) -> Option<DataPoint<HomeStateValue>> {
        let current_value = {
            let current = self.current.borrow();
            current.get(&id).cloned()
        };

        match current_value {
            Some(dp) => Some(dp),
            None => {
                let calculated_value = HomeStateDerivedStateProvider.calculate_current(id, self)?;
                let previous_dp = self.history.get(&id).and_then(|df| df.last());

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

    fn data_frame(&self, id: HomeStateId, range: DateTimeRange) -> Option<DataFrame<HomeStateValue>> {
        let prev_df = self.history.get(&id);
        let current = self.get_home_state_value(id).take_if(|dp| &dp.timestamp <= range.end());

        match (current, prev_df) {
            //TODO optimize to avoid double retain
            (Some(current), Some(df)) => {
                let mut df = df.retain_range_with_context_before(&range);
                df.insert(current.clone());
                Some(df.retain_range_with_context_before(&range))
            }
            (Some(current), None) if current.timestamp <= *range.end() => Some(DataFrame::new(vec![current.clone()])),
            (None, Some(df)) => Some(df.retain_range_with_context_before(&range)),
            _ => None,
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
