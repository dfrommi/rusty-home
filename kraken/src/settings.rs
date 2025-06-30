use config::{Config, ConfigError, File};
use infrastructure::{DatabaseConfig, HttpServerConfig, MonitoringConfig, MqttConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub mqtt: MqttConfig,
    pub homeassistant: crate::homeassistant::HomeAssitant,
    pub z2m: crate::z2m::Zigbee2Mqtt,
    pub tasmota: crate::tasmota::Tasmota,
    pub http_server: HttpServerConfig,
    pub monitoring: MonitoringConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("kraken.toml"))
            .build()?;

        s.try_deserialize()
    }
}
