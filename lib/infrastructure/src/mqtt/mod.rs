mod client;
mod receiver;
mod sender;

pub use client::Mqtt;
pub use receiver::{MqttInMessage, MqttSubscription};
pub use sender::MqttSender;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {
    host: String,
    port: u16,
    client_id: String,
}

impl MqttConfig {
    pub fn new_client(&self) -> Mqtt {
        Mqtt::connect(&self.host, self.port, &self.client_id)
    }
}
