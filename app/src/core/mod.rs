pub mod domain;
pub mod id;
pub mod math;
pub mod range;
pub mod time;
pub mod timeseries;
pub mod unit;

use std::collections::HashMap;

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

    pub fn get_optional(&self, key: &str) -> Option<&[V]> {
        self.config.get(key).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_config_returns_values_for_present_key() {
        let config = DeviceConfig::new(&[("device", 1), ("device", 2)]);

        assert_eq!(config.get_optional("device"), Some([1, 2].as_slice()));
    }

    #[test]
    fn device_config_returns_none_for_missing_key() {
        let config = DeviceConfig::new(&[("device", 1)]);

        assert_eq!(config.get_optional("missing"), None);
    }
}
