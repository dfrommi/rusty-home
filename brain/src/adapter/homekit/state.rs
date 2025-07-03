use std::collections::HashMap;

use api::state::{FanActivity, Powered, unit::FanAirflow};
use infrastructure::MqttOutMessage;
use support::{ExternalId, ValueObject, unit::Percent};
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{core::app_event::StateChangedEvent, home::state::EnergySaving, port::DataPointAccess};

use super::MqttStateValue;

pub async fn export_state<T>(
    api: &T,
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    mut state_changed: Receiver<StateChangedEvent>,
) where
    T: DataPointAccess<Powered> + DataPointAccess<EnergySaving> + DataPointAccess<FanActivity>,
{
    let mut sender = MqttStateSender::new(base_topic.to_owned(), tx);
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = state_changed.recv() => {},
            _ = timer.tick() => {},
        }

        send_with_defaults(&mut sender, Powered::Dehumidifier, api).await;
        send_with_defaults(&mut sender, Powered::InfraredHeater, api).await;
        send_with_defaults(&mut sender, EnergySaving::LivingRoomTv, api).await;
        send_fan_activity(&mut sender, FanActivity::LivingRoomCeilingFan, api).await;
        send_fan_activity(&mut sender, FanActivity::BedroomCeilingFan, api).await;
    }
}

async fn send_with_defaults<'a, 'b: 'a, API, T>(
    sender: &'a mut MqttStateSender,
    state: T,
    api: &'b API,
) where
    T: AsRef<ExternalId> + ValueObject + Clone,
    T::ValueType: Into<MqttStateValue>,
    API: DataPointAccess<T>,
{
    let external_id: &ExternalId = state.as_ref();
    let value = match api.current(state.clone()).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "Error getting current value of {}/{} for sending to MQTT: {:?}",
                external_id.ext_type(),
                external_id.ext_name(),
                e
            );
            return;
        }
    };

    sender.send(external_id, value).await;
}

async fn send_fan_activity<'a, 'b: 'a, API>(
    sender: &'a mut MqttStateSender,
    state: FanActivity,
    api: &'b API,
) where
    API: DataPointAccess<FanActivity>,
{
    let external_id: &ExternalId = state.as_ref();
    let value = match api.current(state.clone()).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "Error getting current value of {}/{} for sending to MQTT: {:?}",
                external_id.ext_type(),
                external_id.ext_name(),
                e
            );
            return;
        }
    };

    let percent_ext_id = ExternalId::new("fan_speed", external_id.ext_name());
    match value {
        FanAirflow::Off => sender.send(&percent_ext_id, Percent(0.0)).await,
        FanAirflow::Forward(fan_speed) => sender.send(&percent_ext_id, fan_speed).await,
        FanAirflow::Reverse(fan_speed) => sender.send(&percent_ext_id, fan_speed).await,
    };
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

    async fn send<'a, T>(&'a mut self, external_id: &ExternalId, value: T)
    where
        T: Into<MqttStateValue>,
    {
        let value: MqttStateValue = value.into();

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
