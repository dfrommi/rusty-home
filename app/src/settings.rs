use config::{Config, ConfigError, Environment, File};
use infrastructure::{DatabaseConfig, HttpServerConfig, MonitoringConfig, MqttConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub mqtt: MqttConfig,
    pub http_server: HttpServerConfig,
    pub monitoring: MonitoringConfig,
    pub homebridge: crate::adapter::homekit::Homekit,
    pub homeassistant: crate::adapter::homeassistant::HomeAssitant,
    pub z2m: Zigbee2MqttSettings,
    pub tasmota: TasmotaSettings,
    pub metrics: crate::adapter::metrics_export::MetricsExport,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let builder = Config::builder()
            .add_source(File::with_name("config.toml"))
            .add_source(Environment::default().separator("_").list_separator(","));

        let s = builder.build()?;
        s.try_deserialize()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TasmotaSettings {
    pub event_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Zigbee2MqttSettings {
    pub event_topic: String,
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    pub struct TestSettings {
        pub live_database: DatabaseConfig,
    }

    impl TestSettings {
        pub fn load() -> Result<Self, ConfigError> {
            let source = match find_file_upwards("test.toml") {
                Some(p) => File::from(p),
                None => return Err(ConfigError::NotFound("test.toml".to_owned())),
            };

            Config::builder().add_source(source).build()?.try_deserialize()
        }
    }

    fn find_file_upwards(file_name: &str) -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;

        // Iterate over ancestors, starting from the current directory
        for dir in current_dir.ancestors() {
            let file_path = dir.join(file_name);
            if file_path.exists() {
                return Some(file_path);
            }
        }

        None
    }
}
