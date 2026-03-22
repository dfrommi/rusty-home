use config::{Config, ConfigError, Environment, File};
use infrastructure::{DatabaseConfig, HttpServerConfig, MonitoringConfig, MqttConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub mqtt: MqttConfig,
    pub http_server: HttpServerConfig,
    pub monitoring: MonitoringConfig,
    pub homebridge: crate::frontends::homekit::Homekit,
    pub homeassistant: HomeAssistantSettings,
    pub z2m: Zigbee2MqttSettings,
    pub tasmota: TasmotaSettings,
    pub metrics: MetricsExportSettings,
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
pub struct HomeAssistantSettings {
    pub topic_event: String,
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TasmotaSettings {
    pub event_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Zigbee2MqttSettings {
    pub event_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsExportSettings {
    pub victoria_url: String,
}
