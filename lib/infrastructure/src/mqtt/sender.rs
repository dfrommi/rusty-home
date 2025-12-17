use std::sync::Arc;

use rumqttc::v5::{AsyncClient, mqttbytes::QoS};

#[derive(Clone)]
pub struct MqttSender {
    client: Arc<AsyncClient>,
}

impl MqttSender {
    pub(super) fn new(client: Arc<AsyncClient>) -> Self {
        Self { client }
    }

    pub async fn send_retained(&self, topic: impl Into<String>, payload: impl Into<String>) -> anyhow::Result<()> {
        self.send(topic.into(), payload.into(), false).await
    }

    pub async fn send_transient(&self, topic: impl Into<String>, payload: impl Into<String>) -> anyhow::Result<()> {
        self.send(topic.into(), payload.into(), false).await
    }

    #[tracing::instrument(skip_all, fields(topic = %topic, otel.name = format!("MQTT publish {}", topic)))]
    async fn send(&self, topic: String, payload: String, retain: bool) -> anyhow::Result<()> {
        tracing::debug!("Publishing MQTT message to {topic} (retain={retain}): {:?}", payload);

        self.client
            .publish(topic.clone(), QoS::ExactlyOnce, retain, payload)
            .await
            .map_err(|e| {
                tracing::error!("Error publishing MQTT message to {}: {}", topic, e);
                e.into()
            })
    }
}
