use std::collections::HashMap;

use crate::core::time::{DateTime, Duration};
use crate::observability::system_metric_set;
use infrastructure::{Mqtt, MqttSender, MqttSubscription, TraceContext};
use serde_json::Value;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Z2mSender {
    tx: mpsc::Sender<Z2mCommandRequest>,
}

pub struct Z2mSenderRunner {
    base_topic: String,
    sender: MqttSender,
    receiver: MqttSubscription,
    cmd_rx: mpsc::Receiver<Z2mCommandRequest>,
    devices: HashMap<String, DeviceTracker>,
}

struct Z2mCommandRequest {
    device_id: String,
    payloads: Vec<Value>,
    optimistic: bool,
    correlation_id: Option<String>,
}

#[derive(Debug, Clone)]
struct DeviceTracker {
    device_id: String,
    payloads: Vec<Value>,
    last_payload_sent: Option<Value>,
    last_payload_sent_at: Option<DateTime>,
    last_state: Value,
    backoff: ExponentialBackoff,
}

fn resend_delay() -> Duration {
    Duration::seconds(5)
}

fn max_backoff_delay() -> Duration {
    Duration::seconds(300)
}

#[derive(Debug, Clone)]
struct ExponentialBackoff {
    attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl ExponentialBackoff {
    fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            attempts: 0,
            base_delay,
            max_delay,
        }
    }

    fn reset(&mut self) {
        self.attempts = 0;
    }

    fn next_delay(&self) -> Duration {
        let base = self.base_delay.as_secs();
        let multiplier = 2i64.saturating_pow(self.attempts.min(31));
        let delay = base.saturating_mul(multiplier).min(self.max_delay.as_secs());
        Duration::seconds(delay)
    }

    fn bump(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
    }
}

impl DeviceTracker {
    fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            payloads: Vec::new(),
            last_payload_sent: None,
            last_payload_sent_at: None,
            last_state: empty_state(),
            backoff: ExponentialBackoff::new(resend_delay(), max_backoff_delay()),
        }
    }

    fn reset_for_payloads(&mut self, payloads: Vec<Value>) {
        self.payloads = payloads;
        self.last_payload_sent = None;
        self.last_payload_sent_at = None;
        self.reset_backoff();
    }

    fn update_state(&mut self, state: Value) {
        self.last_state = state;
    }

    fn next_payload(&self) -> Option<&Value> {
        self.payloads
            .iter()
            .find(|payload| !matches_expected_subset(payload, &self.last_state))
    }

    fn last_payload_matches(&self, payload: &Value) -> bool {
        self.last_payload_sent.as_ref() == Some(payload)
    }

    fn record_send(&mut self, payload: Value) -> Value {
        self.last_payload_sent = Some(payload.clone());
        self.last_payload_sent_at = Some(DateTime::now());
        payload
    }

    fn reset_backoff(&mut self) {
        self.backoff.reset();
        self.record_metric();
    }

    fn bump_backoff(&mut self) {
        self.backoff.bump();
        self.record_metric();
    }

    fn should_delay_resend(&self, payload: &Value) -> Option<Duration> {
        let last_sent_at = self.last_payload_sent_at?;
        if !self.last_payload_matches(payload) {
            return None;
        }

        let delay = self.backoff.next_delay();
        let elapsed = DateTime::now().elapsed_since(last_sent_at);
        if elapsed < delay {
            return Some(delay);
        }

        None
    }

    fn next_payload_to_send(&mut self) -> Option<Value> {
        let Some(next_payload) = self.next_payload().cloned() else {
            self.reset_backoff();
            tracing::debug!(state = %self.last_state, "Z2M sync: state already reflected for all payloads; done");
            return None;
        };

        let is_same_payload = self.last_payload_matches(&next_payload);
        if !is_same_payload {
            self.reset_backoff();
        }

        if let Some(delay) = self.should_delay_resend(&next_payload) {
            tracing::info!(
                command = %next_payload,
                state = %self.last_state,
                "Z2M sync: payload resend delayed by backoff of {}",
                delay
            );
            return None;
        }

        if is_same_payload {
            self.bump_backoff();
        }

        Some(next_payload)
    }

    fn record_metric(&self) {
        system_metric_set(
            "z2m_command_resend_attempts",
            self.backoff.attempts as f64,
            &[("device_id", &self.device_id)],
        );

        system_metric_set(
            "z2m_command_resend_delay_seconds",
            self.backoff.next_delay().as_secs_f64(),
            &[("device_id", &self.device_id)],
        );
    }
}

impl Z2mSender {
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> anyhow::Result<(Self, Z2mSenderRunner)> {
        let base_topic = event_topic.trim_matches('/').to_owned();
        let topic_pattern = format!("{}/#", base_topic);
        let receiver = mqtt_client.subscribe(topic_pattern).await?;
        let (tx, cmd_rx) = mpsc::channel(64);

        Ok((
            Self { tx },
            Z2mSenderRunner {
                base_topic,
                sender: mqtt_client.sender(event_topic),
                receiver,
                cmd_rx,
                devices: HashMap::new(),
            },
        ))
    }

    pub async fn send(&self, device_id: &str, payloads: Vec<Value>, optimistic: bool) -> anyhow::Result<()> {
        if payloads.is_empty() {
            return Err(anyhow::anyhow!(
                "Z2M send received with empty payload list for device {}",
                device_id
            ));
        }

        self.tx
            .send_timeout(
                Z2mCommandRequest {
                    device_id: device_id.to_string(),
                    payloads,
                    optimistic,
                    correlation_id: TraceContext::current_correlation_id(),
                },
                tokio::time::Duration::from_secs(5),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Z2M sender channel closed: {}", e))
    }
}

impl Z2mSenderRunner {
    pub async fn run(mut self) {
        let mut schedule = tokio::time::interval(tokio::time::Duration::from_secs(600));
        let sonoff_devices = vec![
            "bedroom/radiator_thermostat_sonoff",
            "living_room/radiator_thermostat_big_sonoff",
            "living_room/radiator_thermostat_small_sonoff",
            "room_of_requirements/radiator_thermostat_sonoff",
            "kitchen/radiator_thermostat_sonoff",
            "bathroom/radiator_thermostat_sonoff",
        ];

        loop {
            tokio::select! {
                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        tracing::error!("Z2M sender channel closed; stopping runner");
                        break;
                    };
                    TraceContext::continue_from(&cmd.correlation_id);
                    self.handle_command(cmd).await;
                }
                msg = self.receiver.recv() => {
                    let Some(msg) = msg else {
                        continue;
                    };

                    let topic = Z2mTopic::new(&self.base_topic, &msg.topic);
                    self.handle_state(topic, &msg.payload).await;
                }
                _ = schedule.tick() => {
                    for device_id in &sonoff_devices {
                        self.sonoff_thermostat_hack(device_id).await;
                    }
                }
            }
        }
    }

    #[tracing::instrument(name = "sonoff_thermostat_hack", skip(self), fields(%device_id))]
    async fn sonoff_thermostat_hack(&self, device_id: &str) {
        tracing::debug!("Waking up Sonoff thermostat {} actively", device_id);

        // Placeholder for any device-specific hacks or adjustments
        let payload = serde_json::json!({
            "valve_opening_degree": "",
            "valve_closing_degree": "",
            "system_mode": "",
            "occupied_heating_setpoint": "",
        });
        let topic = Z2mTopic::new(&self.base_topic, device_id);

        if let Err(e) = self
            .sender
            .send_transient(topic.active_get_topic(), payload.to_string())
            .await
        {
            tracing::error!("Failed to publish active get for Sonoff thermostat {}: {}", device_id, e);
        }
    }

    #[tracing::instrument(name = "handle_z2m_command", skip(self, cmd), fields(device_id = %cmd.device_id, command = tracing::field::Empty))]
    async fn handle_command(&mut self, cmd: Z2mCommandRequest) {
        if cmd.optimistic {
            tracing::debug!("Z2M optimistic send requested; sending payload list without tracking");
            for payload in cmd.payloads {
                if let Err(e) = self.publish(&cmd.device_id, &payload).await {
                    tracing::error!(command = %payload, "Failed to publish Z2M payload for device {}: {}", cmd.device_id, e);
                    return;
                }
            }
            return;
        }

        let entry = self
            .devices
            .entry(cmd.device_id.clone())
            .or_insert_with(|| DeviceTracker::new(&cmd.device_id));

        entry.reset_for_payloads(cmd.payloads);

        tracing::trace!("Z2M sync: command received; replaced payload list for tracking");

        self.maybe_send_next(&cmd.device_id).await;
    }

    #[tracing::instrument(name = "handle_z2m_state", skip_all, fields(%topic, device_id = tracing::field::Empty, state = tracing::field::Empty))]
    async fn handle_state(&mut self, topic: Z2mTopic, payload: &str) {
        tracing::trace!("Z2M sync: received Z2M state message on topic {}", topic);

        if !topic.is_state_update() {
            tracing::trace!("Z2M sync: ignoring Z2M state message on set topic");
            return;
        }

        let device_id = match topic.device_id() {
            Some(device_id) => {
                TraceContext::record("device_id", &device_id);
                device_id
            }
            None => {
                tracing::warn!(
                    "Z2M sync: failed to extract device ID from topic {}; ignoring Z2M state message",
                    topic
                );
                return;
            }
        };

        let state = match serde_json::from_str::<Value>(payload) {
            Ok(state) => {
                TraceContext::record_json("state", &state);
                state
            }
            Err(e) => {
                TraceContext::record("state", payload);
                tracing::error!("Z2M sync: failed to parse Z2M state payload for device {}: {}", device_id, e);
                return;
            }
        };

        {
            let entry = match self.devices.get_mut(&device_id) {
                Some(entry) => entry,
                None => return,
            };

            tracing::trace!("Z2M sync: processing Z2M state message for device {}", device_id);
            entry.update_state(state);
            tracing::debug!("Z2M sync: state update received; evaluating next send");
        }

        self.maybe_send_next(&device_id).await;
    }

    async fn maybe_send_next(&mut self, device_id: &str) {
        let payload = {
            let Some(entry) = self.devices.get_mut(device_id) else {
                return;
            };
            entry.next_payload_to_send()
        };

        let Some(payload) = payload else {
            return;
        };

        TraceContext::record_json("command", &payload);

        if let Err(e) = self.publish(device_id, &payload).await {
            tracing::error!("Failed to publish Z2M payload for device {}: {}", device_id, e);
            return;
        }

        if let Some(entry) = self.devices.get_mut(device_id) {
            entry.record_send(payload.clone());
        }

        tracing::info!("Z2M payload sent as next step");
    }

    async fn publish(&self, device_id: &str, payload: &Value) -> anyhow::Result<()> {
        tracing::info!("Publishing Z2M command payload to device {}", device_id);
        self.sender
            .send_transient(Z2mTopic::new(&self.base_topic, device_id).command_topic(), payload.to_string())
            .await
    }
}

struct Z2mTopic {
    base_topic: String,
    topic: String,
}

impl Z2mTopic {
    fn new(base_topic: &str, topic: &str) -> Self {
        Self {
            base_topic: base_topic.to_string(),
            topic: topic.to_string(),
        }
    }

    fn is_command(&self) -> bool {
        self.topic.ends_with("/set")
    }

    fn is_state_update(&self) -> bool {
        !self.is_command()
    }

    fn device_id(&self) -> Option<String> {
        self.topic
            .strip_prefix(&self.base_topic)
            .map(|topic| topic.trim_matches('/').to_owned())
            .filter(|device_id| !device_id.is_empty())
    }

    fn command_topic(&self) -> String {
        format!("{}/set", self.topic.trim_matches('/'))
    }

    fn active_get_topic(&self) -> String {
        format!("{}/get", self.topic.trim_matches('/'))
    }
}

impl std::fmt::Display for Z2mTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.topic)
    }
}

fn empty_state() -> Value {
    Value::Object(serde_json::Map::new())
}

fn matches_expected_subset(expected: &Value, actual: &Value) -> bool {
    match expected {
        Value::Object(expected_map) => match actual {
            Value::Object(actual_map) => expected_map.iter().all(|(key, expected_value)| {
                actual_map
                    .get(key)
                    .is_some_and(|actual_value| matches_expected_subset(expected_value, actual_value))
            }),
            _ => false,
        },
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_subset_with_extra_fields() {
        let expected = json!({"state": "ON", "power": 5});
        let actual = json!({"state": "ON", "power": 5, "last_seen": "now"});
        assert!(matches_expected_subset(&expected, &actual));
    }

    #[test]
    fn mismatch_when_key_missing() {
        let expected = json!({"state": "ON", "power": 5});
        let actual = json!({"state": "ON"});
        assert!(!matches_expected_subset(&expected, &actual));
    }

    #[test]
    fn mismatch_when_value_differs() {
        let expected = json!({"state": "ON", "power": 5});
        let actual = json!({"state": "OFF", "power": 5});
        assert!(!matches_expected_subset(&expected, &actual));
    }
}
