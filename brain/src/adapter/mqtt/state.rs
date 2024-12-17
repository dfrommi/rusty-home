use std::collections::HashMap;

use api::{
    state::{ChannelTypeInfo, Powered},
    StateValueAddedEvent,
};
use support::mqtt::MqttOutMessage;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{port::DataPointAccess, state::EnergySaving};

use support::TypedItem;

use super::MqttStateValue;

pub async fn export_state<T>(
    api: &T,
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    mut state_changed: Receiver<StateValueAddedEvent>,
) where
    T: DataPointAccess<Powered> + DataPointAccess<EnergySaving>,
{
    let mut sender = MqttStateSender::new(base_topic.to_owned(), tx);
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = state_changed.recv() => {},
            _ = timer.tick() => {},
        }

        sender.send(Powered::Dehumidifier, api).await;
        sender.send(Powered::InfraredHeater, api).await;
        sender.send(EnergySaving::LivingRoomTv, api).await;
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

    async fn send<'a, 'b: 'a, API, T>(&'a mut self, state: T, api: &'b API)
    where
        T: TypedItem + ChannelTypeInfo + Clone,
        T::ValueType: Into<MqttStateValue>,
        API: DataPointAccess<T>,
    {
        let value = match api.current(state.clone()).await {
            Ok(v) => v.into(),
            Err(e) => {
                tracing::error!(
                    "Error getting current value of {}/{} for sending to MQTT: {:?}",
                    state.type_name(),
                    state.item_name(),
                    e
                );
                return;
            }
        };

        let msg = MqttOutMessage {
            topic: format!(
                "{}/{}/{}",
                self.base_topic,
                state.type_name(),
                state.item_name()
            ),
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
