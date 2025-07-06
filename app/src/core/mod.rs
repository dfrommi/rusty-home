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

use crate::home::state::PersistentStateValue;
use crate::home::trigger::UserTrigger;
pub use command::CommandExecutor;
pub use command::execute_commands;
pub use incoming_data::IncomingDataSource;
pub use incoming_data::process_incoming_data_source;

use time::DateTime;
use timeseries::DataPoint;

pub trait ValueObject {
    type ValueType;
}

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<PersistentStateValue>),
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
