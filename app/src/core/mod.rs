mod api;
pub mod app_event;
pub mod command;
pub mod id;
pub mod incoming_data;
pub mod metrics;
pub mod persistence;
pub mod planner;
pub mod time;
pub mod timeseries;
pub mod unit;

use std::collections::HashMap;

use crate::Infrastructure;
use crate::home::state::PersistentHomeStateValue;
use crate::home::trigger::UserTrigger;
pub use api::HomeApi;
pub use command::CommandExecutor;
pub use incoming_data::IncomingDataSource;
pub use incoming_data::process_incoming_data_source;

#[cfg(test)]
pub use planner::plan_for_home;

use time::DateTime;
use timeseries::DataPoint;

pub trait ValueObject
where
    Self::ValueType: Clone,
{
    type ValueType;

    fn to_f64(value: &Self::ValueType) -> f64;
    fn from_f64(value: f64) -> Self::ValueType;
}

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<PersistentHomeStateValue>),
    UserTrigger(UserTrigger),
    ItemAvailability(ItemAvailability),
}

#[derive(Debug, Clone)]
pub struct ItemAvailability {
    pub source: String,
    pub item: String,
    pub last_seen: DateTime,
    pub marked_offline: bool,
}

pub struct DeviceConfig<V> {
    config: HashMap<String, Vec<V>>,
}

impl<V> DeviceConfig<V>
where
    V: Clone,
{
    pub fn new(config: &[(&str, V)]) -> Self {
        let mut m: HashMap<String, Vec<V>> = HashMap::new();
        for (key, value) in config {
            let key = key.to_string();
            m.entry(key).or_default().push(value.clone());
        }

        Self { config: m }
    }

    pub fn get(&self, key: &str) -> &[V] {
        match self.config.get(key) {
            Some(v) => v,
            None => &[],
        }
    }
}

pub fn plan_and_execute<P: CommandExecutor, S: CommandExecutor>(
    infrastructure: &Infrastructure,
    primary: P,
    secondary: S,
) -> impl Future<Output = ()> + use<P, S> {
    let planner = planner::keep_on_planning(infrastructure);
    let executor = command::keep_command_executor_running(infrastructure, primary, secondary);

    async move {
        tokio::select! {
            _ = planner => {},
            _ = executor => {},
        }
    }
}
