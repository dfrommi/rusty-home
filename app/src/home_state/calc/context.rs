use std::{cell::RefCell, collections::HashMap};

use infrastructure::TraceContext;

use crate::{
    core::{
        id::ExternalId,
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateId, DeviceStateItem, DeviceStateValue},
    home_state::{HomeStateDerivedStateProvider, HomeStateId, HomeStateItem, HomeStateValue},
    t,
    trigger::{UserTriggerExecution, UserTriggerTarget},
};

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

#[derive(Clone)]
pub struct StateCalculationResult {
    start_time: DateTime,
    data: HashMap<HomeStateId, DataFrame<HomeStateValue>>,
    active_user_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
}

impl Default for StateCalculationResult {
    fn default() -> Self {
        StateCalculationResult {
            start_time: t!(now),
            data: HashMap::new(),
            active_user_triggers: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for StateCalculationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCalculationResult")
            .field("start_time", &self.start_time)
            .field("data_keys", &self.data.keys().collect::<Vec<&HomeStateId>>())
            .field(
                "active_user_triggers_keys",
                &self.active_user_triggers.keys().collect::<Vec<&UserTriggerTarget>>(),
            )
            .finish()
    }
}

impl StateCalculationResult {
    pub fn timestamp(&self) -> DateTime {
        self.start_time
    }

    pub fn get_home_state_value(&self, id: HomeStateId) -> Option<DataPoint<HomeStateValue>> {
        self.data.get(&id).and_then(|df| df.last().cloned())
    }

    pub fn data_frame(&self, id: HomeStateId, since: DateTime) -> Option<DataFrame<HomeStateValue>> {
        self.data.get(&id).map(|df| df.get_since_plus_one(since))
    }

    pub fn user_trigger(&self, target: UserTriggerTarget) -> Option<&UserTriggerExecution> {
        self.active_user_triggers.get(&target)
    }

    pub fn home_state_iter(&self) -> impl Iterator<Item = (&HomeStateId, &DataFrame<HomeStateValue>)> {
        self.data.iter()
    }

    fn truncate_to(&mut self, cutoff: DateTime) {
        for (_, df) in self.data.iter_mut() {
            df.remove_before_keep_one_more(cutoff);
        }

        self.data.retain(|_, df| !df.is_empty());
        self.start_time = cutoff;
    }
}

pub struct StateCalculationContext {
    start_time: DateTime,
    current: RefCell<HashMap<HomeStateId, DataPoint<HomeStateValue>>>,
    device_state: Box<dyn DeviceStateProvider>,
    active_user_triggers: Box<dyn UserTriggerProvider>,
    prev: StateCalculationResult,
    trace_contexts: HashMap<String, TraceContext>,
}

impl StateCalculationContext {
    pub fn new<D: DeviceStateProvider, T: UserTriggerProvider>(
        device_state: D,
        active_user_triggers: T,
        mut previous: StateCalculationResult,
        keep: Duration,
        enable_tracing: bool,
    ) -> Self {
        let start_time = t!(now);
        let cutoff = start_time - keep;

        previous.truncate_to(cutoff);

        let mut trace_contexts: HashMap<String, TraceContext> = HashMap::new();
        if enable_tracing && let Some(root_span) = TraceContext::current() {
            for home_state_id in HomeStateId::variants().iter() {
                let ext_id_type = home_state_id.ext_id().type_name().to_string();
                if trace_contexts.contains_key(&ext_id_type) {
                    continue;
                }

                let span = tracing::trace_span!("calculate_home_state_type", otel.name = ext_id_type.as_str());
                root_span.make_parent_of(&span);
                trace_contexts.insert(ext_id_type.to_string(), TraceContext::for_span(span));
            }
        }

        StateCalculationContext {
            start_time,
            current: RefCell::new(HashMap::new()),
            device_state: Box::new(device_state),
            active_user_triggers: Box::new(active_user_triggers),
            prev: previous,
            trace_contexts,
        }
    }

    pub fn timestamp(&self) -> DateTime {
        self.start_time
    }

    #[tracing::instrument(name = "load_all_home_states", skip(self))]
    pub fn load_all(&self) {
        for id in HomeStateId::variants().iter() {
            self.get_home_state_value(*id);
        }
    }

    #[tracing::instrument(name = "into_calculation_result", skip(self))]
    pub fn into_result(self) -> StateCalculationResult {
        let current_data = self.current.into_inner();
        let mut data = self.prev.data;

        for (id, dp) in current_data.into_iter() {
            let df = data.entry(id).or_insert_with(DataFrame::empty);
            df.insert(dp);
        }

        StateCalculationResult {
            start_time: self.start_time,
            data,
            active_user_triggers: self.active_user_triggers.get_all(),
        }
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

    pub fn trace(&self, id: impl Into<ExternalId>, item_name: &'static str, value: impl Into<f64>) {
        if DateTime::is_shifted() {
            //skip tracing in shifted time contexts
            return;
        }

        let ext_context_id: ExternalId = id.into();

        infrastructure::meter::set(
            "home_state_calculation",
            value.into(),
            &[("item", item_name), ("home_state_id", &ext_context_id.to_string())],
        );
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
                let calculated_value = self.calculate_new_home_state_value(id)?;
                let previous_dp = self.prev.get_home_state_value(id);

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

    fn calculate_new_home_state_value(&self, id: HomeStateId) -> Option<HomeStateValue> {
        let ext_id: ExternalId = id.into();

        //TODO maybe enter root?
        let span = if let Some(root_ctx) = self.trace_contexts.get(ext_id.type_name()) {
            let span = tracing::trace_span!(
                "calculate_home_state",
                otel.name = tracing::field::Empty,
                home_state_id = id.ext_id().to_string(),
                cached = false
            );
            root_ctx.make_parent_of(&span);
            span
        } else {
            tracing::Span::none()
        };

        let _enter = span.enter();

        let calculated_value = HomeStateDerivedStateProvider.calculate_current(id, self);

        if let Some(ref value) = calculated_value {
            //TODO display
            TraceContext::set_current_span_name(format!("{} - {:?}", ext_id.variant_name(), value));
        } else {
            TraceContext::set_current_span_name(format!("{} - none", ext_id.variant_name()));
        }

        calculated_value
    }

    fn data_frame(&self, id: HomeStateId, since: DateTime) -> Option<DataFrame<HomeStateValue>> {
        if self.start_time < since {
            return None;
        }

        let mut df = self.prev.data_frame(id, since).unwrap_or_default();

        if let Some(dp) = self.get_home_state_value(id) {
            df.insert(dp);
        }

        if df.is_empty() { None } else { Some(df) }
    }
}
