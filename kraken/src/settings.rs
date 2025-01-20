use config::{Config, ConfigError, File};
use infrastructure::{DatabaseConfig, HttpServerConfig, MonitoringConfig, MqttConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub mqtt: MqttConfig,
    pub homeassistant: HomeAssitant,
    pub z2m: Zigbee2Mqtt,
    pub tasmota: Tasmota,
    pub http_server: HttpServerConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HomeAssitant {
    pub topic_event: String,
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Zigbee2Mqtt {
    pub event_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Tasmota {
    pub event_topic: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("kraken.toml"))
            .build()?;

        s.try_deserialize()
    }
}
