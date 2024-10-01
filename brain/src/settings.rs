use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub database: Database,
    pub mqtt: Mqtt,
}

#[derive(Debug, Deserialize)]
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
    pub base_topic_status: String,
    pub base_topic_set: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("brain.toml"))
            .build()?;

        s.try_deserialize()
    }
}
