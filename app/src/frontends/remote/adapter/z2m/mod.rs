mod config;

use crate::{
    core::DeviceConfig,
    trigger::{DualButtonPress, RemoteTrigger, RemoteTriggerTarget},
};
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};
use serde::Deserialize;

pub struct Z2mRemoteIncomingDataSource {
    device_config: DeviceConfig<RemoteTriggerTarget>,
    mqtt_receiver: MqttSubscription,
}

impl Z2mRemoteIncomingDataSource {
    #[allow(clippy::expect_used)]
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> Self {
        let mqtt_receiver = mqtt_client
            .subscribe(event_topic, "#")
            .await
            .expect("Error subscribing to MQTT topic");

        Self {
            device_config: DeviceConfig::new(&config::default_z2m_remote_config()),
            mqtt_receiver,
        }
    }

    pub async fn recv_multi(&mut self) -> Option<Vec<RemoteTrigger>> {
        loop {
            let msg = self.mqtt_receiver.recv().await?;

            let Some(device_id) = self.device_id(&msg) else {
                continue;
            };

            let targets = self.device_config.get(&device_id);
            if targets.is_empty() {
                continue;
            }

            match self.parse_triggers(msg.payload, targets) {
                Ok(Some(triggers)) => return Some(triggers),
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!("Error parsing Z2M remote payload for {}: {:?}", device_id, e);
                }
            }
        }
    }

    fn device_id(&self, msg: &MqttInMessage) -> Option<String> {
        if msg.topic.ends_with("/set") {
            return None;
        }

        Some(msg.topic.clone())
    }

    fn parse_triggers(
        &self,
        payload: String,
        targets: &[RemoteTriggerTarget],
    ) -> anyhow::Result<Option<Vec<RemoteTrigger>>> {
        #[derive(Deserialize)]
        struct Payload {
            action: Option<String>,
        }

        let payload: Payload = serde_json::from_str(&payload)?;

        let Some(action) = payload.action else {
            return Ok(None);
        };

        let Some(button_press) = parse_button_press(action.as_str()) else {
            return Ok(None);
        };

        let triggers = targets
            .iter()
            .map(|target| match target {
                RemoteTriggerTarget::BedroomDoorRemote => RemoteTrigger::BedroomDoorRemote(button_press.clone()),
            })
            .collect::<Vec<_>>();

        Ok(Some(triggers))
    }
}

fn parse_button_press(action: &str) -> Option<DualButtonPress> {
    match action {
        "on" => Some(DualButtonPress::SingleOn),
        "brightness_move_up" => Some(DualButtonPress::HoldOn),
        "off" => Some(DualButtonPress::SingleOff),
        "brightness_move_down" => Some(DualButtonPress::HoldOff),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_action() {
        assert!(matches!(parse_button_press("off"), Some(DualButtonPress::SingleOff)));
        assert!(matches!(
            parse_button_press("brightness_move_up"),
            Some(DualButtonPress::HoldOn)
        ));
    }

    #[test]
    fn ignore_unknown_action() {
        assert!(parse_button_press("toggle").is_none());
    }
}
