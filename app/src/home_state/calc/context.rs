use std::{cell::RefCell, collections::HashMap};

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateClient, DeviceStateId, DeviceStateItem, DeviceStateValue},
    home_state::{HomeState, HomeStateDerivedStateProvider, StateValue},
    port::ValueObject,
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
    for id in HomeState::variants().iter() {
        context.load(id.clone());
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
    let triggers = trigger_client.get_all_active_triggers().await?;
    let mut active_triggers = HashMap::new();

    for trigger in triggers {
        active_triggers.insert(trigger.target(), trigger);
    }

    let mut history_map: HashMap<HomeState, DataFrame<StateValue>> = HashMap::new();
    for (id, value) in history.home_state_iter() {
        history_map.insert(id.clone(), value.clone());
    }

    let device_states = device_state.get_current_for_all().await?;

    Ok(StateCalculationContext {
        current: RefCell::new(HashMap::new()),
        history: history_map,
        device_state: device_states,
        active_user_triggers: active_triggers,
    })
}

pub struct StateCalculationContext {
    current: RefCell<HashMap<HomeState, DataPoint<StateValue>>>,
    history: HashMap<HomeState, DataFrame<StateValue>>,
    device_state: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>,
    active_user_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
}

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<DataPoint<T>>;
}

impl StateCalculationContext {
    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject + Clone,
    {
        let state_value = self.get_home_state_value(id.clone().into())?;
        let value = id.project_state_value(state_value.value)?;

        Some(DataPoint {
            value,
            timestamp: state_value.timestamp,
        })
    }

    fn get_home_state_value(&self, id: HomeState) -> Option<DataPoint<StateValue>> {
        let current_value = {
            let current = self.current.borrow();
            current.get(&id).cloned()
        };

        match current_value {
            Some(dp) => Some(dp),
            None => {
                let now = t!(now);
                let mut calculated_dp = HomeStateDerivedStateProvider.calculate_current(id.clone(), self)?;
                if calculated_dp.timestamp > now {
                    calculated_dp.timestamp = now;
                } else if let Some(last_history_ts) = self.last_history_timestamp(&id)
                    && calculated_dp.timestamp <= last_history_ts
                {
                    calculated_dp.timestamp = now;
                }

                let mut current_mut = self.current.borrow_mut();
                current_mut.insert(id.clone(), calculated_dp.clone());
                Some(calculated_dp)
            }
        }
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

    fn last_history_timestamp(&self, id: &HomeState) -> Option<DateTime> {
        self.history.get(id).and_then(|df| df.last()).map(|dp| dp.timestamp)
    }

    //combine previous snapshot and current value into full frame
    //TODO dataframe slices and use reference
    pub fn all_of_last<S>(&self, id: S, duration: Duration) -> Option<DataFrame<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject + Clone,
    {
        let df = self.data_frame(id.clone().into(), DateTimeRange::new(t!(now) - duration, t!(now)))?;
        downcast_df(id, &df)
    }

    pub fn all_since<S>(&self, id: S, timestamp: DateTime) -> Option<DataFrame<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject + Clone,
    {
        let df = self.data_frame(id.clone().into(), DateTimeRange::since(timestamp))?;
        downcast_df(id, &df)
    }

    fn data_frame(&self, id: HomeState, range: DateTimeRange) -> Option<DataFrame<StateValue>> {
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

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.active_user_triggers.get(&target)
    }

    fn load(&self, id: HomeState) {
        let _ = self.get_home_state_value(id);
    }

    fn into_snapshot(self, range: DateTimeRange) -> StateSnapshot {
        let mut data = HashMap::new();

        for (id, current) in self.current.borrow().iter() {
            let df: Option<DataFrame<StateValue>> = self.history.get(id).cloned();
            let combined_df = match df {
                Some(mut df) => {
                    df.insert(current.clone());
                    df.retain_range_with_context_before(&range)
                }
                None => DataFrame::new(vec![current.clone()]),
            };
            data.insert(id.clone(), combined_df);
        }

        StateSnapshot::new(data, self.active_user_triggers)
    }
}

fn downcast_df<S: ValueObject + Into<HomeState>>(id: S, df: &DataFrame<StateValue>) -> Option<DataFrame<S::ValueType>> {
    Some(df.map(|dp: &DataPoint<StateValue>| {
        //TODO fail gracefully
        id.project_state_value(dp.value.clone())
            .expect("Internal error: State value projection failed in downcast_df")
    }))
}
