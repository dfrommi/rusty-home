use std::sync::Arc;

use rumqttc::v5::{AsyncClient, mqttbytes::QoS};

#[derive(Clone)]
pub struct MqttSender {
    client: Arc<AsyncClient>,
    base_topic: String,
}

impl MqttSender {
    pub(super) fn new(client: Arc<AsyncClient>, base_topic: impl Into<String>) -> Self {
        Self {
            client,
            base_topic: base_topic.into(),
        }
    }

    pub async fn send_retained(&self, topic: impl Into<String>, payload: impl Into<String>) -> anyhow::Result<()> {
        self.send(topic.into(), payload.into(), false).await
    }

    pub async fn send_transient(&self, topic: impl Into<String>, payload: impl Into<String>) -> anyhow::Result<()> {
        self.send(topic.into(), payload.into(), false).await
    }

    #[tracing::instrument(skip_all, fields(topic = %topic, otel.name = format!("MQTT publish {}", topic)))]
    async fn send(&self, topic: String, payload: String, retain: bool) -> anyhow::Result<()> {
        let topic = Self::join_topic(&self.base_topic, &topic);
        tracing::debug!("Publishing MQTT message to {topic} (retain={retain}): {:?}", payload);

        self.client
            .publish(topic.clone(), QoS::ExactlyOnce, retain, payload)
            .await
            .map_err(|e| {
                tracing::error!("Error publishing MQTT message to {}: {}", topic, e);
                e.into()
            })
    }

    fn join_topic(base_topic: &str, topic: &str) -> String {
        let base_topic = base_topic.trim_matches('/');
        let topic = topic.trim_matches('/');

        match (base_topic.is_empty(), topic.is_empty()) {
            (true, true) => String::new(),
            (false, true) => base_topic.to_string(),
            (true, false) => topic.to_string(),
            (false, false) => format!("{base_topic}/{topic}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MqttSender;

    #[test]
    fn join_topic_uses_single_separator() {
        assert_eq!(MqttSender::join_topic("home/base/", "/device/set"), "home/base/device/set");
    }

    #[test]
    fn join_topic_handles_missing_separators() {
        assert_eq!(MqttSender::join_topic("home/base", "device/set"), "home/base/device/set");
    }

    #[test]
    fn join_topic_handles_empty_base() {
        assert_eq!(MqttSender::join_topic("", "/device/set/"), "device/set");
    }
}
