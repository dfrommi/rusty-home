use config::{Config, ConfigError, File};
use infrastructure::{DatabaseConfig, HttpServerConfig, MonitoringConfig, MqttConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub mqtt: MqttConfig,
    pub http_server: HttpServerConfig,
    pub monitoring: MonitoringConfig,
    pub homekit: crate::adapter::homekit::Homekit,
    pub homeassistant: crate::adapter::homeassistant::HomeAssitant,
    pub z2m: crate::adapter::z2m::Zigbee2Mqtt,
    pub tasmota: crate::adapter::tasmota::Tasmota,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("brain.toml"))
            .build()?;

        s.try_deserialize()
    }
}

#[cfg(test)]
pub mod test {
    use support::file::find_file_upwards;

    use super::*;

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

            Config::builder()
                .add_source(source)
                .build()?
                .try_deserialize()
        }
    }
}
