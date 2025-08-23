use std::collections::HashMap;

use crate::adapter::homekit::{Homekit, HomekitState};
use crate::core::HomeApi;
use crate::core::unit::Percent;
use crate::home::state::FanAirflow;
use infrastructure::MqttOutMessage;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{core::app_event::StateChangedEvent, port::DataPointAccess};

use super::HomekitStateValue;

pub async fn export_state(
    api: &HomeApi,
    base_topic: String,
    tx: Sender<MqttOutMessage>,
    mut state_changed: Receiver<StateChangedEvent>,
) {
    let mut sender = MqttStateSender::new(base_topic.to_owned(), tx);
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = state_changed.recv() => {},
            _ = timer.tick() => {},
        }

        for (key, accessory, _) in Homekit::config() {
            if let Err(e) = export_accessory(key, accessory, api, &mut sender).await {
                tracing::error!("Error exporting to Homekit {}: {:?}", key, e);
            }
        }
    }
}

async fn export_accessory(
    key: &str,
    accessory: HomekitState,
    api: &HomeApi,
    sender: &mut MqttStateSender,
) -> anyhow::Result<()> {
    match accessory {
        HomekitState::Powered(powered) => sender.send(key, powered.current(api).await?).await,
        HomekitState::EnergySaving(energy_saving) => sender.send(key, energy_saving.current(api).await?).await,
        HomekitState::FanSpeed(fan_activity) => {
            match fan_activity.current(api).await? {
                FanAirflow::Off => sender.send(key, Percent(0.0)).await,
                FanAirflow::Forward(fan_speed) | FanAirflow::Reverse(fan_speed) => sender.send(key, fan_speed).await,
            };
        }
    }

    Ok(())
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

    async fn send<T>(&mut self, item: &str, value: T)
    where
        T: Into<HomekitStateValue>,
    {
        let value: HomekitStateValue = value.into();

        let topic = format!("{}/{}", self.base_topic, item);
        let payload = value.0;

        if self.last_sent.get(&topic) == Some(&payload) {
            return;
        }

        let msg = MqttOutMessage::retained(topic.clone(), payload.clone());

        self.tx.send(msg).await.unwrap();
        self.last_sent.insert(topic, payload);
    }
}
