use std::collections::HashMap;

use crate::core::time::{DateTime, Duration};
use infrastructure::{Mqtt, MqttInMessage, MqttSender, MqttSubscription};
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
}

#[derive(Debug, Clone)]
struct DeviceTracker {
    payloads: Vec<Value>,
    last_payload_sent: Option<Value>,
    last_payload_sent_at: Option<DateTime>,
    last_state: Value,
    consecutive_no_progress: u32,
    halted: bool,
}

const MAX_NO_PROGRESS: u32 = 10;

fn resend_delay() -> Duration {
    Duration::seconds(5)
}

impl DeviceTracker {
    fn new() -> Self {
        Self {
            payloads: Vec::new(),
            last_payload_sent: None,
            last_payload_sent_at: None,
            last_state: empty_state(),
            consecutive_no_progress: 0,
            halted: false,
        }
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
                sender: mqtt_client.sender(),
                receiver,
                cmd_rx,
                devices: HashMap::new(),
            },
        ))
    }

    pub async fn send(&self, device_id: &str, payloads: Vec<Value>, optimistic: bool) -> anyhow::Result<()> {
        self.tx
            .send_timeout(
                Z2mCommandRequest {
                    device_id: device_id.to_string(),
                    payloads,
                    optimistic,
                },
                tokio::time::Duration::from_secs(5),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Z2M sender channel closed: {}", e))
    }
}

impl Z2mSenderRunner {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        tracing::error!("Z2M sender channel closed; stopping runner");
                        break;
                    };
                    self.handle_command(cmd).await;
                }
                msg = self.receiver.recv() => {
                    let Some(msg) = msg else {
                        continue;
                    };
                    self.handle_state(msg).await;
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: Z2mCommandRequest) {
        if cmd.payloads.is_empty() {
            tracing::warn!(device_id = %cmd.device_id, "Z2M send received with empty payload list; skipping");
            return;
        }

        if cmd.optimistic {
            tracing::debug!(%cmd.device_id, "Z2M optimistic send requested; sending payload list without tracking");
            for payload in cmd.payloads {
                if let Err(e) = self.publish(&cmd.device_id, &payload).await {
                    tracing::error!(%cmd.device_id, command = %payload, "Failed to publish Z2M payload for device {}: {}", cmd.device_id, e);
                    return;
                }
            }
            return;
        }

        let entry = self
            .devices
            .entry(cmd.device_id.clone())
            .or_insert_with(DeviceTracker::new);

        entry.payloads = cmd.payloads;
        entry.last_payload_sent = None;
        entry.last_payload_sent_at = None;
        entry.consecutive_no_progress = 0;
        entry.halted = false;

        tracing::trace!(device_id = %cmd.device_id, "Z2M command received; replaced payload list for tracking");

        self.maybe_send_next(&cmd.device_id, true, false).await;
    }

    async fn handle_state(&mut self, msg: MqttInMessage) {
        tracing::trace!(topic = %msg.topic, "Received Z2M state message on topic {}", msg.topic);

        if is_set_topic(&msg.topic) {
            tracing::trace!(topic = %msg.topic, "Ignoring Z2M state message on set topic");
            return;
        }

        let device_id = match device_id_from_topic(&self.base_topic, &msg.topic) {
            Some(device_id) => device_id,
            None => {
                tracing::warn!(topic = %msg.topic, "Failed to extract device ID from topic; ignoring Z2M state message");
                return;
            }
        };

        let entry = match self.devices.get_mut(&device_id) {
            Some(entry) => entry,
            None => return,
        };

        tracing::trace!(%device_id, "Processing Z2M state message for device {}", device_id);

        let state = match serde_json::from_str::<Value>(&msg.payload) {
            Ok(state) => state,
            Err(e) => {
                tracing::error!(%device_id, "Failed to parse Z2M state payload for device {}: {}", device_id, e);
                return;
            }
        };

        let state_unchanged = entry.last_state == state;
        entry.last_state = state;

        if state_unchanged && entry.consecutive_no_progress > 0 {
            tracing::debug!(%device_id, state = %entry.last_state, "Z2M state unchanged since last update; evaluating next send");
        } else {
            tracing::debug!(%device_id, state = %entry.last_state, "Z2M state updated; evaluating next send");
        }

        self.maybe_send_next(&device_id, false, state_unchanged).await;
    }

    async fn maybe_send_next(&mut self, device_id: &str, from_send: bool, state_unchanged: bool) {
        let Some((next_payload, state_snapshot)) = self.prepare_next_payload(device_id, from_send, state_unchanged)
        else {
            return;
        };

        if let Err(e) = self.publish(device_id, &next_payload).await {
            tracing::error!(%device_id, command = %next_payload, "Failed to publish Z2M payload for device {}: {}", device_id, e);
            return;
        }

        if let Some(entry) = self.devices.get_mut(device_id) {
            entry.last_payload_sent = Some(next_payload.clone());
            entry.last_payload_sent_at = Some(DateTime::now());
            entry.consecutive_no_progress = 0;
        }

        tracing::info!(%device_id, command = %next_payload, state = %state_snapshot, "Z2M payload sent as next step");
    }

    fn prepare_next_payload(
        &mut self,
        device_id: &str,
        from_send: bool,
        state_unchanged: bool,
    ) -> Option<(Value, Value)> {
        let entry = self.devices.get_mut(device_id)?;

        if entry.payloads.is_empty() {
            tracing::info!(%device_id, "Z2M payload list empty; skipping send evaluation");
            return None;
        }

        if entry.halted {
            tracing::info!(%device_id, "Z2M sender halted for device; skipping send evaluation");
            return None;
        }

        let next_payload = entry
            .payloads
            .iter()
            .find(|payload| !matches_expected_subset(payload, &entry.last_state));

        let Some(next_payload) = next_payload else {
            entry.consecutive_no_progress = 0;
            tracing::debug!(%device_id, state = %entry.last_state, "Z2M state already reflects all payloads; no send needed");
            return None;
        };

        if !from_send {
            if state_unchanged && entry.last_payload_sent.as_ref() == Some(next_payload) {
                entry.consecutive_no_progress += 1;
                if entry.consecutive_no_progress >= MAX_NO_PROGRESS {
                    entry.halted = true;
                    tracing::info!(
                        %device_id,
                        command = %next_payload,
                        state = %entry.last_state,
                        "Z2M sender halted after {} unchanged updates with pending payload",
                        entry.consecutive_no_progress
                    );
                    return None;
                }
            } else {
                entry.consecutive_no_progress = 0;
            }
        }

        if entry.last_payload_sent.as_ref() == Some(next_payload)
            && let Some(last_sent_at) = entry.last_payload_sent_at
        {
            let delay = resend_delay();
            if DateTime::now().elapsed_since(last_sent_at) < delay {
                tracing::info!(
                    %device_id,
                    command = %next_payload,
                    state = %entry.last_state,
                    "Z2M payload was sent within {} seconds; delaying resend",
                    delay.as_secs()
                );
                return None;
            }
        }

        Some((next_payload.clone(), entry.last_state.clone()))
    }

    async fn publish(&self, device_id: &str, payload: &Value) -> anyhow::Result<()> {
        tracing::info!(%device_id, command = %payload, "Publishing Z2M command payload to device {}", device_id);
        self.sender
            .send_transient(target_topic(&self.base_topic, device_id), payload.to_string())
            .await
    }
}

fn target_topic(base_topic: &str, device_id: &str) -> String {
    format!("{}/{}/set", base_topic, device_id)
}

fn device_id_from_topic(base_topic: &str, topic: &str) -> Option<String> {
    topic
        .strip_prefix(base_topic)
        .map(|topic| topic.trim_matches('/').to_owned())
        .filter(|device_id| !device_id.is_empty())
}

fn is_set_topic(topic: &str) -> bool {
    topic.ends_with("/set")
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
