mod config;

use crate::{
    core::DeviceConfig,
    trigger::{DimmerSwitchButtonPress, RemoteTrigger, RemoteTriggerTarget},
};
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};
use serde::Deserialize;

pub struct HomeAssistantRemoteIncomingDataSource {
    device_config: DeviceConfig<RemoteTriggerTarget>,
    mqtt_receiver: MqttSubscription,
}

impl HomeAssistantRemoteIncomingDataSource {
    #[allow(clippy::expect_used)]
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> Self {
        let mqtt_receiver = mqtt_client
            .subscribe(event_topic, "")
            .await
            .expect("Error subscribing to Home Assistant MQTT topic");

        Self {
            device_config: DeviceConfig::new(&config::default_ha_remote_config()),
            mqtt_receiver,
        }
    }

    pub async fn recv_multi(&mut self) -> Option<Vec<RemoteTrigger>> {
        loop {
            let msg = self.mqtt_receiver.recv().await?;

            let event = match parse_event(&msg) {
                Ok(Some(event)) => event,
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!("Error parsing Home Assistant remote payload: {:?}", e);
                    continue;
                }
            };

            let Some(targets) = self.device_config.get_optional(&event.device_id) else {
                continue;
            };

            let Some(button_press) = parse_button_press(&event.event_type, event.subtype) else {
                continue;
            };

            return Some(
                targets
                    .iter()
                    .filter_map(|target| match target {
                        RemoteTriggerTarget::BedframeRemote => {
                            Some(RemoteTrigger::BedframeRemote(button_press.clone()))
                        }
                        RemoteTriggerTarget::BedroomDoorRemote => None,
                    })
                    .collect(),
            );
        }
    }
}

#[derive(Debug)]
struct HomeAssistantRemoteEvent {
    device_id: String,
    event_type: String,
    subtype: u8,
}

fn parse_event(msg: &MqttInMessage) -> anyhow::Result<Option<HomeAssistantRemoteEvent>> {
    #[derive(Deserialize)]
    struct Payload {
        event_type: String,
        event_data: serde_json::Value,
    }

    #[derive(Deserialize)]
    struct EventData {
        id: String,
        r#type: String,
        subtype: u8,
    }

    let payload: Payload = serde_json::from_str(&msg.payload)?;

    if payload.event_type != "hue_event" {
        return Ok(None);
    }

    let event_data: EventData = serde_json::from_value(payload.event_data)?;
    Ok(Some(HomeAssistantRemoteEvent {
        device_id: event_data.id,
        event_type: event_data.r#type,
        subtype: event_data.subtype,
    }))
}

fn parse_button_press(event_type: &str, subtype: u8) -> Option<DimmerSwitchButtonPress> {
    let button_press = match subtype {
        1 => DimmerSwitchButtonPress::SingleOn,
        2 => DimmerSwitchButtonPress::SingleDimUp,
        3 => DimmerSwitchButtonPress::SingleDimDown,
        4 => DimmerSwitchButtonPress::SingleOff,
        _ => return None,
    };

    match event_type {
        "initial_press" => Some(button_press),
        "repeat" | "long_press" => Some(to_hold(button_press)),
        "short_release" | "long_release" => None,
        _ => None,
    }
}

fn to_hold(button_press: DimmerSwitchButtonPress) -> DimmerSwitchButtonPress {
    match button_press {
        DimmerSwitchButtonPress::SingleOn => DimmerSwitchButtonPress::HoldOn,
        DimmerSwitchButtonPress::SingleDimUp => DimmerSwitchButtonPress::HoldDimUp,
        DimmerSwitchButtonPress::SingleDimDown => DimmerSwitchButtonPress::HoldDimDown,
        DimmerSwitchButtonPress::SingleOff => DimmerSwitchButtonPress::HoldOff,
        _ => button_press,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(payload: &str) -> MqttInMessage {
        MqttInMessage {
            topic: "".to_string(),
            payload: payload.to_string(),
        }
    }

    #[test]
    fn parses_hue_event() {
        let event = parse_event(&message(
            r#"{"event_type":"hue_event","event_data":{"id":"dimmer_bett_button","type":"initial_press","subtype":4}}"#,
        ))
        .unwrap();

        let event = event.unwrap();
        assert_eq!(event.device_id, "dimmer_bett_button");
        assert_eq!(event.event_type, "initial_press");
        assert_eq!(event.subtype, 4);
    }

    #[test]
    fn ignores_non_hue_events() {
        assert!(
            parse_event(&message(r#"{"event_type":"state_changed","event_data":{}}"#))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn maps_initial_press_to_single_button_press() {
        assert!(matches!(
            parse_button_press("initial_press", 4),
            Some(DimmerSwitchButtonPress::SingleOff)
        ));
        assert!(matches!(
            parse_button_press("initial_press", 2),
            Some(DimmerSwitchButtonPress::SingleDimUp)
        ));
    }

    #[test]
    fn maps_hold_events_to_hold_button_press() {
        assert!(matches!(parse_button_press("repeat", 1), Some(DimmerSwitchButtonPress::HoldOn)));
        assert!(matches!(
            parse_button_press("long_press", 3),
            Some(DimmerSwitchButtonPress::HoldDimDown)
        ));
    }

    #[test]
    fn ignores_release_and_unknown_button_events() {
        assert!(parse_button_press("short_release", 4).is_none());
        assert!(parse_button_press("long_release", 4).is_none());
        assert!(parse_button_press("initial_press", 5).is_none());
        assert!(parse_button_press("unknown", 4).is_none());
    }
}
