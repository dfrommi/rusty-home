use std::{cell::RefCell, collections::HashMap};

use crate::{
    core::{
        HomeApi,
        persistence::UserTriggerRequest,
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    home::{
        state::{HomeState, HomeStateDerivedStateProvider, PersistentHomeState, StateValue},
        trigger::UserTriggerTarget,
    },
    port::ValueObject,
    t,
};

use super::StateSnapshot;

pub async fn calculate_new_snapshot(
    range: DateTimeRange,
    history: StateSnapshot,
    api: &HomeApi,
) -> anyhow::Result<StateSnapshot> {
    tracing::debug!("Calculating new state snapshot for range {:?}...", range);
    let context = create_new_calculation_context(history, api).await?;

    //preload all current values
    tracing::debug!("Preloading current state values...");
    for id in HomeState::variants().iter() {
        if !id.is_persistent() {
            context.load(id.clone());
        }
    }

    let snapshot = context.into_snapshot(range);
    Ok(snapshot)
}

//TODO optimize creation of context using less copy and loops, ideally Arc
pub async fn create_new_calculation_context(
    history: StateSnapshot,
    api: &HomeApi,
) -> anyhow::Result<StateCalculationContext> {
    let triggers = api.all_user_triggers_since(t!(48 hours ago)).await?;
    let mut active_triggers = HashMap::new();

    for trigger in triggers {
        active_triggers.insert(trigger.target(), trigger);
    }

    //TODO optimize loading of persistent states, ideally all in one query
    let mut persistent_states: HashMap<HomeState, DataFrame<StateValue>> = HashMap::new();
    let state_range = DateTimeRange::since(t!(8 hours ago));
    for item in PersistentHomeState::variants().iter() {
        let df = match api.get_data_frame(item, state_range.clone()).await {
            Err(e) => {
                tracing::error!("Error loading persistent state {:?}: {:?}", item, e);
                continue;
            }
            Ok(df) => df.map(|dp| dp.value.value().into()),
        };
        persistent_states.insert(HomeState::from(item.clone()), df);
    }

    let mut history_map: HashMap<HomeState, DataFrame<StateValue>> = HashMap::new();
    for (id, value) in history.home_state_iter() {
        if !id.is_persistent() {
            history_map.insert(id.clone(), value.clone());
        }
    }

    Ok(StateCalculationContext {
        now: t!(now),
        current: RefCell::new(HashMap::new()),
        history: history_map,
        persistent_state: persistent_states,
        active_user_triggers: active_triggers,
    })
}

pub struct StateCalculationContext {
    now: DateTime,
    //Has to contain persistent current values
    current: RefCell<HashMap<HomeState, DataPoint<StateValue>>>,
    history: HashMap<HomeState, DataFrame<StateValue>>,
    persistent_state: HashMap<HomeState, DataFrame<StateValue>>,
    active_user_triggers: HashMap<UserTriggerTarget, UserTriggerRequest>,
}

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<DataPoint<T>>;
}

impl StateCalculationContext {
    pub fn get<S>(&self, id: S) -> Option<DataPoint<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject,
    {
        let state_value = self.get_home_state_value(id.into())?;
        let value = S::project_state_value(state_value.value)?;

        Some(DataPoint {
            value,
            timestamp: state_value.timestamp,
        })
    }

    fn get_home_state_value(&self, id: HomeState) -> Option<DataPoint<StateValue>> {
        if id.is_persistent() {
            return self.persistent_state.get(&id).map(|df| df.last().clone());
        }

        let current_value = {
            let current = self.current.borrow();
            current.get(&id).cloned()
        };

        match current_value {
            Some(dp) => Some(dp),
            None => {
                let mut calculated_dp = HomeStateDerivedStateProvider.calculate_current(id.clone(), self)?;
                if calculated_dp.timestamp > self.now {
                    calculated_dp.timestamp = self.now;
                } else if let Some(last_history_ts) = self.last_history_timestamp(&id)
                    && calculated_dp.timestamp <= last_history_ts
                {
                    calculated_dp.timestamp = self.now;
                }

                let mut current_mut = self.current.borrow_mut();
                current_mut.insert(id.clone(), calculated_dp.clone());
                Some(calculated_dp)
            }
        }
    }

    fn last_history_timestamp(&self, id: &HomeState) -> Option<DateTime> {
        self.history.get(id).map(|df| df.last().timestamp)
    }

    //combine previous snapshot and current value into full frame
    //TODO dataframe slices and use reference
    pub fn all_of_last<S>(&self, id: S, duration: Duration) -> Option<DataFrame<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject,
    {
        let df = self.data_frame(id.into(), DateTimeRange::new(self.now - duration, self.now))?;
        downcast_df::<S>(&df)
    }

    pub fn all_since<S>(&self, id: S, timestamp: DateTime) -> Option<DataFrame<S::ValueType>>
    where
        S: Into<HomeState> + ValueObject,
    {
        let df = self.data_frame(id.into(), DateTimeRange::new(timestamp, self.now))?;
        downcast_df::<S>(&df)
    }

    fn data_frame(&self, id: HomeState, range: DateTimeRange) -> Option<DataFrame<StateValue>> {
        if id.is_persistent() {
            return self
                .persistent_state
                .get(&id)
                .and_then(|df| df.retain_range_with_context_before(&range));
        }

        let prev_df = self.history.get(&id);
        let current = self.get_home_state_value(id).take_if(|dp| &dp.timestamp <= range.end());

        match (current, prev_df) {
            //TODO optimize to avoid double retain
            (Some(current), Some(df)) => {
                let mut df = df.retain_range_with_context_before(&range)?;
                df.insert(current.clone());
                df.retain_range_with_context_before(&range)
            }
            (Some(current), None) if current.timestamp <= *range.end() => DataFrame::new(vec![current.clone()]).ok(),
            (None, Some(df)) => df.retain_range_with_context_before(&range),
            _ => None,
        }
    }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerRequest> {
        self.active_user_triggers.get(&target)
    }

    fn load(&self, id: HomeState) {
        let _ = self.get_home_state_value(id);
    }

    fn into_snapshot(self, range: DateTimeRange) -> StateSnapshot {
        let mut data = HashMap::new();

        for (id, current) in self.current.borrow().iter() {
            let df = self.history.get(id).cloned();
            let combined_df = match df {
                Some(mut df) => {
                    df.insert(current.clone());
                    df.retain_range_with_context_before(&range)
                        .expect("Internal error: Error retaining range in data frame of non-empty datapoints")
                }
                None => DataFrame::new(vec![current.clone()])
                    .expect("Internal error: Error creating data frame of non-empty datapoints"),
            };
            data.insert(id.clone(), combined_df);
        }

        for (key, df) in self.persistent_state.iter() {
            data.insert(key.clone(), df.clone());
        }

        StateSnapshot::new(data, self.active_user_triggers)
    }
}

fn downcast_df<S: ValueObject + Into<HomeState>>(df: &DataFrame<StateValue>) -> Option<DataFrame<S::ValueType>> {
    Some(df.map(|dp: &DataPoint<StateValue>| {
        //TODO fail gracefully
        S::project_state_value(dp.value.clone()).expect("Internal error: State value projection failed in downcast_df")
    }))
}
