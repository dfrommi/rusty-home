use std::collections::HashMap;

use super::Z2mTopic;
use crate::core::math::round_to_one_decimal;
use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::home_state::{HomeStateEvent, HomeStateValue};
use crate::observability::system_metric_set;
use crate::{automation::Radiator, core::resilience::ExponentialBackoff};
use infrastructure::{EventListener, Mqtt, MqttSender, MqttSubscription, TraceContext};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::Level;

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
    sonoff_devices: Vec<SonoffThermostatCoreSync>,
    home_state_events: EventListener<HomeStateEvent>,
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
    last_state: Value,
    backoff: ExponentialBackoff,
}

fn resend_delay() -> Duration {
    Duration::seconds(5)
}

fn max_backoff_delay() -> Duration {
    Duration::seconds(300)
}

impl DeviceTracker {
    fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            payloads: Vec::new(),
            last_payload_sent: None,
            last_state: empty_state(),
            backoff: ExponentialBackoff::new(resend_delay(), max_backoff_delay()),
        }
    }

    fn reset_for_payloads(&mut self, payloads: Vec<Value>) {
        self.payloads = payloads;
        self.last_payload_sent = None;
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

    fn should_delay_resend(&self, payload: &Value) -> bool {
        if !self.last_payload_matches(payload) {
            return false;
        }

        !self.backoff.may_retry()
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

        if self.should_delay_resend(&next_payload) {
            tracing::info!(
                command = %next_payload,
                state = %self.last_state,
                "Z2M sync: payload resend delayed",
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
            self.backoff.attempts() as f64,
            &[("device_id", &self.device_id)],
        );
    }
}

impl Z2mSender {
    pub async fn new(
        mqtt_client: &mut Mqtt,
        event_topic: &str,
        home_state_events: EventListener<HomeStateEvent>,
    ) -> anyhow::Result<(Self, Z2mSenderRunner)> {
        let base_topic = event_topic.trim_matches('/').to_owned();
        let topic_pattern = format!("{}/#", base_topic);
        let receiver = mqtt_client.subscribe(topic_pattern).await?;
        let (tx, cmd_rx) = mpsc::channel(64);
        let sender = mqtt_client.sender(event_topic);

        let sonoff_devices = Radiator::variants()
            .iter()
            .map(|&radiator| SonoffThermostatCoreSync::new(radiator, sender.clone()))
            .collect();

        Ok((
            Self { tx },
            Z2mSenderRunner {
                base_topic,
                receiver,
                sender,
                cmd_rx,
                devices: HashMap::new(),
                sonoff_devices,
                home_state_events,
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

struct SonoffThermostatCoreSync {
    radiator: Radiator,
    mqtt_sender: MqttSender,
    device_id: String,
    topic: Z2mTopic,
}

impl SonoffThermostatCoreSync {
    fn new(radiator: Radiator, mqtt_sender: MqttSender) -> Self {
        let device_id = match radiator {
            Radiator::Bedroom => "bedroom/radiator_thermostat_sonoff",
            Radiator::LivingRoomBig => "living_room/radiator_thermostat_big_sonoff",
            Radiator::LivingRoomSmall => "living_room/radiator_thermostat_small_sonoff",
            Radiator::RoomOfRequirements => "room_of_requirements/radiator_thermostat_sonoff",
            Radiator::Kitchen => "kitchen/radiator_thermostat_sonoff",
            Radiator::Bathroom => "bathroom/radiator_thermostat_sonoff",
        };

        Self {
            radiator,
            mqtt_sender,
            device_id: device_id.to_string(),
            topic: Z2mTopic::new(device_id),
        }
    }

    #[tracing::instrument(name = "sonoff_thermostat_hack", skip(self), fields(device_id = %self.device_id))]
    async fn sonoff_keep_alive(&self) {
        tracing::debug!(device_id = %self.device_id, "Waking up Sonoff thermostat {} actively", self.device_id);

        // Placeholder for any device-specific hacks or adjustments
        let payload = serde_json::json!({
            "valve_opening_degree": "",
            "valve_closing_degree": "",
            "system_mode": "",
            "occupied_heating_setpoint": "",
        });

        if let Err(e) = self
            .mqtt_sender
            .send_transient(self.topic.active_get_topic(), payload.to_string())
            .await
        {
            tracing::error!(device_id = %self.device_id, "Failed to publish active get for Sonoff thermostat {}: {}", self.device_id, e);
        }
    }

    #[tracing::instrument(level = Level::TRACE, name = "sonoff_set_temperature", skip(self, event), fields(device_id = %self.device_id))]
    async fn handle_home_state_event(&self, event: &HomeStateEvent) {
        match event {
            HomeStateEvent::Changed(DataPoint {
                value: HomeStateValue::Temperature(id, temp),
                ..
            }) if *id == self.radiator.room_temperature() => {
                let temp = round_to_one_decimal(temp.0);

                tracing::debug!(device_id = %self.device_id, "External temperature update for {}: temperature {}", self.device_id, temp);
                let payload = serde_json::json!({
                    "external_temperature_input": temp,
                    "temperature_sensor_select": "external"
                });

                self.mqtt_sender.send_transient(self.topic.command_topic(), payload.to_string()).await.unwrap_or_else(|e| {
                    tracing::error!(device_id = %self.device_id, "Failed to publish temperature update for Sonoff thermostat {}: {}", self.device_id, e);
                });
            }
            _ => { /* Ignore other events */ }
        }
    }
}

impl Z2mSenderRunner {
    pub async fn run(mut self) {
        let mut schedule = tokio::time::interval(tokio::time::Duration::from_secs(600));

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

                    self.handle_state(&msg.topic, &msg.payload).await;
                }
                event = self.home_state_events.recv() => {
                    let Some(event) = event else {
                        continue;
                    };

                    for sonoff_device in &self.sonoff_devices {
                        sonoff_device.handle_home_state_event(&event).await;
                    }
                }
                _ = schedule.tick() => {
                    for sonoff_device in &self.sonoff_devices {
                        sonoff_device.sonoff_keep_alive().await;
                    }
                }
            }
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

    #[tracing::instrument(name = "handle_z2m_state", skip_all, fields(topic = %mqtt_topic, device_id = tracing::field::Empty, state = tracing::field::Empty))]
    async fn handle_state(&mut self, mqtt_topic: &str, payload: &str) {
        tracing::trace!("Z2M sync: received Z2M state message on topic {}", mqtt_topic);

        if !Z2mTopic::is_state_update(mqtt_topic) {
            tracing::trace!("Z2M sync: ignoring Z2M state message on set topic");
            return;
        }

        let Some(topic) = self.topic_from_incoming(mqtt_topic) else {
            tracing::warn!(
                "Z2M sync: failed to extract device ID from topic {}; ignoring Z2M state message",
                mqtt_topic
            );
            return;
        };

        let device_id = topic.device_id().to_string();
        TraceContext::record("device_id", &device_id);

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
            .send_transient(Z2mTopic::new(device_id).command_topic(), payload.to_string())
            .await
    }

    fn topic_from_incoming(&self, mqtt_topic: &str) -> Option<Z2mTopic> {
        mqtt_topic
            .trim_matches('/')
            .strip_prefix(&self.base_topic)
            .map(|topic| topic.trim_matches('/'))
            .filter(|topic| !topic.is_empty())
            .and_then(Z2mTopic::from_topic)
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
