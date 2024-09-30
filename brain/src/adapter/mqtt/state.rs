use std::collections::HashMap;

use crate::error::Result;
use api::state::Powered;
use support::{mqtt::MqttOutMessage, unit::PowerState};
use tokio::sync::mpsc::Sender;

use crate::prelude::DataPointAccess;

pub async fn process(base_topic: &str, tx: Sender<MqttOutMessage>) {
    let mut sender = MqttStateSender::new(base_topic.to_string(), tx);

    loop {
        sender.send(Powered::Dehumidifier).await;

        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

struct MqttStateSender {
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    last_sent: HashMap<String, String>,
}

impl MqttStateSender {
    fn new(base_topic: String, tx: Sender<MqttOutMessage>) -> Self {
        Self {
            base_topic,
            tx,
            last_sent: HashMap::new(),
        }
    }

    async fn send(&mut self, state: impl IntoMqttState) {
        let state = state.into_mqtt_state().await.unwrap();
        let msg = MqttOutMessage {
            topic: format!("{}/{}/{}/get", self.base_topic, state.name, state.channel),
            payload: state.payload,
            retain: true,
        };

        if self.last_sent.get(&msg.topic) == Some(&msg.payload) {
            return;
        }

        self.tx.send(msg.clone()).await.unwrap();
        self.last_sent.insert(msg.topic, msg.payload);
    }
}

struct MqttState {
    name: String,
    channel: String,
    payload: String,
}

struct MqttStateValue(String);

trait IntoMqttState {
    async fn into_mqtt_state(self) -> Result<MqttState>;
}

impl IntoMqttState for Powered {
    async fn into_mqtt_state(self) -> Result<MqttState> {
        let (name, channel) = match self {
            Powered::Dehumidifier => ("dehumidifier", "power"),
        };

        let v: MqttStateValue = self.current().await?.into();

        Ok(MqttState {
            name: name.to_string(),
            channel: channel.to_string(),
            payload: v.0,
        })
    }
}

impl From<PowerState> for MqttStateValue {
    fn from(val: PowerState) -> Self {
        MqttStateValue(if val.is_on() {
            "1".to_string()
        } else {
            "0".to_string()
        })
    }
}
