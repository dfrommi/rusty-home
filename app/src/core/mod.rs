mod api;
pub mod app_event;
pub mod command;
pub mod id;
pub mod persistence;
pub mod planner;
pub mod state;
pub mod time;
pub mod timeseries;
pub mod unit;

use std::collections::HashMap;

pub use api::HomeApi;
pub use planner::keep_on_planning;

#[cfg(test)]
pub use planner::plan_for_home;

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
