use std::collections::HashMap;

use api::state::Powered;
use infrastructure::mqtt::MqttOutMessage;
use support::{ExternalId, ValueObject};
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{core::event::StateChangedEvent, home::state::EnergySaving, port::DataPointAccess};

use super::MqttStateValue;

pub async fn export_state<T>(
    api: &T,
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    mut state_changed: Receiver<StateChangedEvent>,
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
        T: AsRef<ExternalId> + ValueObject + Clone,
        T::ValueType: Into<MqttStateValue>,
        API: DataPointAccess<T>,
    {
        let value = match api.current(state.clone()).await {
            Ok(v) => v.into(),
            Err(e) => {
                let external_id: &ExternalId = state.as_ref();
                tracing::error!(
                    "Error getting current value of {}/{} for sending to MQTT: {:?}",
                    external_id.ext_type(),
                    external_id.ext_name(),
                    e
                );
                return;
            }
        };

        let external_id: &ExternalId = state.as_ref();
        let topic = format!(
            "{}/{}/{}",
            self.base_topic,
            external_id.ext_type(),
            external_id.ext_name()
        );
        let payload = value.0;

        if self.last_sent.get(&topic) == Some(&payload) {
            return;
        }

        let msg = MqttOutMessage::retained(topic.clone(), payload.clone());

        self.tx.send(msg).await.unwrap();
        self.last_sent.insert(topic, payload);
    }
}
