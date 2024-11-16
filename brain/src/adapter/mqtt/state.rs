use std::collections::HashMap;

use api::state::Powered;
use support::mqtt::MqttOutMessage;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::port::DataPointAccess;

pub async fn export_state<T>(
    api: &T,
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    mut state_changed: Receiver<()>,
) where
    T: DataPointAccess<Powered>,
{
    let mut sender = MqttStateSender::new(base_topic.to_owned(), tx);
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = state_changed.recv() => {},
            _ = timer.tick() => {},
        }

        sender.send(Powered::Dehumidifier, api).await;
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

    async fn send<'a, 'b: 'a, T>(&'a mut self, state: impl IntoMqttStateId, api: &'b T)
    where
        T: DataPointAccess<Powered>,
    {
        //let state = state.into_mqtt_state(api).await.unwrap();
        let id = state.into_mqtt_state_id().await.unwrap();
        let value = MqttStateValue::from(
            api.current_data_point(Powered::Dehumidifier)
                .await
                .unwrap()
                .value,
        );

        let msg = MqttOutMessage {
            topic: format!("{}/{}/{}", self.base_topic, id.name, id.channel),
            payload: value.0,
            retain: true,
        };

        if self.last_sent.get(&msg.topic) == Some(&msg.payload) {
            return;
        }

        self.tx.send(msg.clone()).await.unwrap();
        self.last_sent.insert(msg.topic, msg.payload);
    }
}

struct MqttStateId {
    name: String,
    channel: String,
}
struct MqttStateValue(String);

trait IntoMqttStateId {
    async fn into_mqtt_state_id(&self) -> Option<MqttStateId>;
}

impl IntoMqttStateId for Powered {
    async fn into_mqtt_state_id(&self) -> Option<MqttStateId> {
        let (name, channel) = match self {
            Powered::Dehumidifier => ("dehumidifier", "power"),
            _ => return None,
        };

        Some(MqttStateId {
            name: name.to_string(),
            channel: channel.to_string(),
        })
    }
}

impl From<bool> for MqttStateValue {
    fn from(val: bool) -> Self {
        MqttStateValue(if val {
            "1".to_string()
        } else {
            "0".to_string()
        })
    }
}
