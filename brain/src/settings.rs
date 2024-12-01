use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub database: Database,
    pub mqtt: Mqtt,
    pub http_server: HttpServer,
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

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HttpServer {
    pub port: u16,
}

#[cfg(test)]
pub mod test {
    use support::file::find_file_upwards;

    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct TestSettings {
        pub live_database: Database,
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
