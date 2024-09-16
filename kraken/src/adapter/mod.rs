use chrono::{DateTime, Utc};
use tokio::sync::mpsc::{Receiver, Sender};

use api::{state::ChannelValue, BackendApi};

mod homeassistant;

pub enum IncomingMessage {
    HomeAssistant { payload: String },
}

pub enum OutgoingMessage {
    HomeAssistant { payload: String },
}

pub struct PersistentDataPoint {
    value: ChannelValue,
    timestamp: DateTime<Utc>,
}

pub async fn init(evt_tx: &Sender<IncomingMessage>, url: &str, token: &str) {
    homeassistant::init(evt_tx, url, token)
        .await
        .expect("Error initializing homeassistant states");
}

pub async fn process_incoming_events(api: &BackendApi, mut rx: Receiver<IncomingMessage>) {
    while let Some(message) = rx.recv().await {
        match <Option<PersistentDataPoint>>::from(message) {
            Some(event) => {
                api.add_thing_value(&event.value, &event.timestamp)
                    .await
                    .expect("Error saving value");
            }
            None => tracing::debug!("Unsupported message received"),
        }
    }
}

pub async fn process_pending_commands(api: &BackendApi, tx: Sender<OutgoingMessage>) {
    loop {
        let command = api.get_command_for_processing().await;

        match command {
            Ok(Some(cmd)) => match homeassistant::to_command_payload(&cmd) {
                Some(payload) => tx
                    .send(OutgoingMessage::HomeAssistant { payload })
                    .await
                    .expect("Error sending message: {payload}"),
                None => tracing::error!("Command not supported by backend: {:?}", cmd),
            },
            Ok(None) => tokio::time::sleep(tokio::time::Duration::from_secs(5)).await,
            Err(e) => {
                tracing::error!("Error getting pending commands: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await
            }
        }
    }
}

impl From<IncomingMessage> for Option<PersistentDataPoint> {
    fn from(value: IncomingMessage) -> Self {
        match value {
            IncomingMessage::HomeAssistant { payload } => {
                homeassistant::to_smart_home_event(&payload)
            }
        }
    }
}
