use config::{Config, ConfigError, File};
use monitoring::MonitoringConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub database: Database,
    pub mqtt: Mqtt,
    pub homeassistant: HomeAssitant,
    pub z2m: Zigbee2Mqtt,
    pub tasmota: Tasmota2Mqtt,
    pub http_server: HttpServer,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Mqtt {
    pub host: String,
    pub port: u16,
    pub client_id: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HttpServer {
    pub port: u16,
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
pub struct Tasmota2Mqtt {
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
