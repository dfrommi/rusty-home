use std::str::Utf8Error;

use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttInMessage {
    pub topic: String,
    pub payload: String,
}

pub struct MqttSubscription {
    rx: mpsc::Receiver<MqttInMessage>,
}

impl MqttSubscription {
    pub(super) fn new(rx: mpsc::Receiver<MqttInMessage>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<MqttInMessage> {
        self.rx.recv().await
    }
}

impl TryInto<MqttInMessage> for &rumqttc::v5::mqttbytes::v5::Publish {
    type Error = Utf8Error;

    fn try_into(self) -> Result<MqttInMessage, Self::Error> {
        Ok(MqttInMessage {
            topic: std::str::from_utf8(&self.topic)?.to_string(),
            payload: std::str::from_utf8(&self.payload)?.to_string(),
        })
    }
}
