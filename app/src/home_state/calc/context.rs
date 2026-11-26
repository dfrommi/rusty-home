use std::{cell::RefCell, collections::HashMap};

use infrastructure::TraceContext;

use crate::{
    core::{
        id::ExternalId,
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
    },
    device_state::{DeviceStateClient, DeviceStateId, DeviceStateItem},
    home_state::{HomeStateDerivedStateProvider, HomeStateId, HomeStateItem, HomeStateValue},
    t,
    trigger::{UserTriggerExecution, UserTriggerTarget},
};

pub trait DerivedStateProvider<ID, T> {
    fn calculate_current(&self, id: ID, context: &StateCalculationContext) -> Option<T>;
}

#[derive(Clone)]
pub struct StateCalculationResult {
    start_time: DateTime,
    data: HashMap<HomeStateId, DataFrame<HomeStateValue>>,
}

impl Default for StateCalculationResult {
    fn default() -> Self {
        StateCalculationResult {
            start_time: t!(now),
            data: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for StateCalculationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCalculationResult")
            .field("start_time", &self.start_time)
            .field("data_keys", &self.data.keys().collect::<Vec<&HomeStateId>>())
            .finish()
    }
}

impl StateCalculationResult {
    pub fn new(start_time: DateTime, data: HashMap<HomeStateId, DataFrame<HomeStateValue>>) -> Self {
        StateCalculationResult { start_time, data }
    }

    pub fn timestamp(&self) -> DateTime {
        self.start_time
    }

    pub fn get_latest(&self, id: HomeStateId) -> Option<DataPoint<HomeStateValue>> {
        self.data.get(&id).and_then(|df| df.last().cloned())
    }

    pub fn data_frame(&self, id: HomeStateId, since: DateTime) -> Option<DataFrame<HomeStateValue>> {
        self.data.get(&id).map(|df| df.get_since_plus_one(since))
    }
}

pub struct StateCalculationContext {
    start_time: DateTime,
    device_state_client: DeviceStateClient,
    current: RefCell<HashMap<HomeStateId, DataPoint<HomeStateValue>>>,
    calculating: RefCell<Vec<HomeStateId>>,
    latest_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
    prev: StateCalculationResult,
    trace_contexts: HashMap<String, TraceContext>,
}

/// Removes the id from the in-progress stack on drop, so an early return
/// (e.g. `calculate_new_home_state_value` yielding `None`) does not leave a
/// stale marker behind that would cause a false cycle detection later.
struct CalculatingGuard<'a> {
    id: HomeStateId,
    stack: &'a RefCell<Vec<HomeStateId>>,
}

impl Drop for CalculatingGuard<'_> {
    fn drop(&mut self) {
        self.stack.borrow_mut().retain(|x| x != &self.id);
    }
}

impl StateCalculationContext {
    pub fn new(
        device_state_client: DeviceStateClient,
        latest_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
        previous: StateCalculationResult,
    ) -> Self {
        let start_time = t!(now);

        let mut trace_contexts: HashMap<String, TraceContext> = HashMap::new();
        if Self::tracing_enabled() {
            let root_span = TraceContext::current();

            for home_state_id in HomeStateId::variants().iter() {
                let ext_id_type = home_state_id.ext_id().type_name().to_string();
                if trace_contexts.contains_key(&ext_id_type) {
                    continue;
                }

                let span = tracing::trace_span!("calculate_home_state_type", otel.name = ext_id_type.as_str());
                root_span.make_parent_of(&span);
                trace_contexts.insert(ext_id_type.to_string(), TraceContext::for_span(&span));
            }
        }

        StateCalculationContext {
            start_time,
            device_state_client,
            current: RefCell::new(HashMap::new()),
            calculating: RefCell::new(Vec::new()),
            latest_triggers,
            prev: previous,
            trace_contexts,
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
                tracing::error!("Error converting home state {:?} to expected type: {}", state_value.value, e);
                None
            }
        }
    }

    #[allow(clippy::expect_used)]
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

    #[allow(clippy::expect_used)]
    pub fn device_states<D>(&self, id: D, since: DateTime) -> Option<DataFrame<D::Type>>
    where
        D: Into<DeviceStateId> + DeviceStateItem + Clone,
    {
        let device_state_id = id.clone().into();
        let df = self
            .device_state_client
            .get_state_since(&device_state_id, since)
            .ok()??;

        Some(df.map(|dp| {
            id.try_downcast(dp.value.clone())
                .expect("Internal error: Device state projection failed in device_states")
        }))
    }

    pub fn active_trigger(&self, target: UserTriggerTarget) -> Option<UserTriggerExecution> {
        self.latest_trigger(target).take_if(|exec| exec.is_active())
    }

    pub fn latest_trigger(&self, target: UserTriggerTarget) -> Option<UserTriggerExecution> {
        self.latest_triggers.get(&target).cloned()
    }

    pub fn device_state<D>(&self, id: D) -> Option<DataPoint<D::Type>>
    where
        D: Into<DeviceStateId> + DeviceStateItem + Clone,
    {
        let device_state_id: DeviceStateId = id.clone().into();
        let dp = self
            .device_state_client
            .get_latest_for_device(&device_state_id)
            .ok()
            .flatten()?;

        match id.try_downcast(dp.value.clone()) {
            Ok(v) => Some(DataPoint::new(v, dp.timestamp)),
            Err(e) => {
                tracing::error!("Error converting device state {:?} to expected type: {}", dp.value, e);
                None
            }
        }
    }

    pub fn trace(&self, id: impl Into<ExternalId>, item_name: &'static str, value: impl Into<f64>) {
        if !Self::tracing_enabled() {
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
    fn tracing_enabled() -> bool {
        !DateTime::is_shifted()
    }

    fn get_home_state_value(&self, id: HomeStateId) -> Option<DataPoint<HomeStateValue>> {
        if let Some(dp) = self.current.borrow().get(&id).cloned() {
            return Some(dp);
        }

        // Re-entering an id that is still being calculated is a dependency cycle.
        {
            let mut calculating = self.calculating.borrow_mut();
            if calculating.contains(&id) {
                tracing::error!(
                    "Dependency cycle detected: {:?} is already being calculated (chain: {:?})",
                    id,
                    calculating
                );
                return None;
            }
            calculating.push(id);
        }

        let _guard = CalculatingGuard {
            id,
            stack: &self.calculating,
        };

        let calculated_value = self.calculate_new_home_state_value(id)?;
        let previous_dp = self.prev.get_latest(id);

        //check if previous value is the same, then reuse timestamp
        let calculated_dp = if let Some(previous_dp) = previous_dp
            && previous_dp.value == calculated_value
        {
            previous_dp.clone()
        } else {
            //no or different previous value
            DataPoint::new(calculated_value, self.start_time)
        };

        self.current.borrow_mut().insert(id, calculated_dp.clone());
        Some(calculated_dp)
    }

    fn calculate_new_home_state_value(&self, id: HomeStateId) -> Option<HomeStateValue> {
        let ext_id: ExternalId = id.into();

        //TODO maybe enter root?
        let span = if let Some(root_ctx) = self.trace_contexts.get(ext_id.type_name()) {
            let span = tracing::trace_span!(
                "calculate_home_state",
                otel.name = ext_id.variant_name(),
                home_state_id = id.ext_id().to_string(),
                cached = false,
            );
            root_ctx.make_parent_of(&span);
            span
        } else {
            tracing::Span::none()
        };
        let ctx = TraceContext::for_span(&span);
        let _enter = span.enter();

        let calculated_value = HomeStateDerivedStateProvider.calculate_current(id, self);

        if let Some(ref value) = calculated_value {
            ctx.record("value", value.to_value_string());
        } else {
            tracing::warn!("Calculated home state value for {:?} is None", id);
            ctx.set_error("Calculated value is None");
            ctx.record("value", "None");
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
