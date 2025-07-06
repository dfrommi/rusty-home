use infrastructure::MqttInMessage;
use serde::Deserialize;

use crate::adapter::homeassistant::StateChangedEvent;

pub struct HaMqttClient {
    state_rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
}

impl HaMqttClient {
    pub fn new(rx: tokio::sync::mpsc::Receiver<MqttInMessage>) -> Self {
        Self { state_rx: rx }
    }
}

impl HaMqttClient {
    pub async fn recv(&mut self) -> Option<StateChangedEvent> {
        match self.state_rx.recv().await {
            Some(msg) => {
                match serde_json::from_str::<HaEvent>(&msg.payload) {
                    Ok(HaEvent::StateChanged {
                        new_state: event, ..
                    }) => {
                        return Some(event);
                    }
                    Ok(HaEvent::Unknown(_)) => {
                        tracing::trace!("Received unsupported event: {:?}", msg.payload);
                        None
                    }

                    //json parsing error
                    Err(e) => {
                        tracing::error!("Error parsing MQTT message: {}", e);
                        None
                    }
                }
            }

            None => {
                tracing::error!("Error parsing MQTT message: channel closed");
                None
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "event_type", content = "event_data")]
#[allow(dead_code)]
pub enum HaEvent {
    #[serde(rename = "state_changed")]
    StateChanged {
        entity_id: String,
        new_state: StateChangedEvent,
    },

    #[serde(untagged)]
    Unknown(serde_json::Value),
}
