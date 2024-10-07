use std::collections::HashMap;

use parse::{HaEvent, StateValue};
use serde_json::Value;
use support::mqtt::MqttInMessage;
use support::unit::present_state::PresentState;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::adapter::config::ha_incoming_event_channel;
use crate::adapter::StateCollector;
use crate::adapter::{homeassistant::event::parse::StateChangedEvent, PersistentDataPoint};
use anyhow::{bail, Result};
use api::state::ChannelValue;
use support::unit::{DegreeCelsius, KiloWattHours, OpenedState, Percent, PowerState, Watt};

use super::HaChannel;

pub struct HaStateCollector {
    api_url: String,
    api_token: String,
    event_rx: mpsc::Receiver<MqttInMessage>,
}

impl HaStateCollector {
    pub fn new(api_url: &str, api_token: &str, event_rx: mpsc::Receiver<MqttInMessage>) -> Self {
        Self {
            api_url: api_url.to_owned(),
            api_token: api_token.to_owned(),
            event_rx,
        }
    }
}

impl StateCollector for HaStateCollector {
    async fn process(mut self, dp_tx: &mpsc::Sender<PersistentDataPoint>) -> Result<()> {
        self.persist_current_state(dp_tx).await?;

        tracing::info!("Start processing HA events");

        while let Some(msg) = self.event_rx.recv().await {
            match serde_json::from_str(&msg.payload) {
                Ok(event) => match to_smart_home_event(&event) {
                    Some(dp) => {
                        if let Err(e) = dp_tx.send(dp).await {
                            tracing::error!(
                                "Error sending data-point to channel for processing: {}",
                                e
                            );
                        }
                    }

                    //unsupported entity
                    None => {
                        tracing::trace!("Unsupported event {:?} received", event);
                    }
                },

                //json parsing error
                Err(e) => tracing::error!("Error parsing MQTT message: {}", e),
            }
        }

        Ok(())
    }
}

impl HaStateCollector {
    pub async fn persist_current_state(
        &self,
        dp_tx: &mpsc::Sender<PersistentDataPoint>,
    ) -> Result<()> {
        info!("Persisting current HA states");
        for event in get_current_states(&self.api_url, &self.api_token).await? {
            if let Some(dp) = to_smart_home_event(&event) {
                dp_tx.send(dp).await?;
            }
        }

        Ok(())
    }
}

fn to_smart_home_event(event: &HaEvent) -> Option<PersistentDataPoint> {
    match event {
        HaEvent::StateChanged {
            entity_id,
            new_state,
            ..
        } => {
            let ha_channel = match ha_incoming_event_channel(entity_id as &str) {
                Some(c) => c,
                None => {
                    debug!("Skipped {}", entity_id);
                    return None;
                }
            };

            match &new_state.state {
                StateValue::Available(state_value) => {
                    info!("Received supported event {}", entity_id);

                    let dp_result = to_persistent_data_point(
                        ha_channel,
                        state_value,
                        &new_state.attributes,
                        new_state.last_changed,
                    );

                    match dp_result {
                        Ok(dp) => Some(dp),
                        Err(e) => {
                            tracing::error!(
                                "Error processing homeassistant event of {}: {:?}",
                                entity_id,
                                e
                            );
                            None
                        }
                    }
                }
                _ => {
                    warn!("Value of {} is not available", entity_id);
                    None
                }
            }
        }
        HaEvent::Unknown(_) => {
            debug!("Received unsupported event");
            None
        }
    }
}

async fn get_current_states(url: &str, token: &str) -> Result<Vec<HaEvent>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/states", url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let events = response.json::<Vec<StateChangedEvent>>().await?;

    tracing::info!("{} init-events ready to send", events.len());

    let result: Vec<HaEvent> = events
        .into_iter()
        .map(|event| HaEvent::StateChanged {
            entity_id: event.entity_id.clone(),
            new_state: event,
        })
        .collect();

    Ok(result)
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    attributes: &HashMap<String, Value>,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<PersistentDataPoint> {
    let dp = match channel {
        HaChannel::Temperature(channel) => PersistentDataPoint {
            value: ChannelValue::Temperature(channel, DegreeCelsius(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::RelativeHumidity(channel) => PersistentDataPoint {
            value: ChannelValue::RelativeHumidity(channel, Percent(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::Opened(channel) => PersistentDataPoint {
            value: ChannelValue::Opened(
                channel,
                if ha_value == "on" {
                    OpenedState::Opened
                } else {
                    OpenedState::Closed
                },
            ),
            timestamp,
        },
        HaChannel::Powered(channel) => PersistentDataPoint {
            value: ChannelValue::Powered(
                channel,
                if ha_value == "on" {
                    PowerState::On
                } else {
                    PowerState::Off
                },
            ),
            timestamp,
        },
        HaChannel::CurrentPowerUsage(channel) => PersistentDataPoint {
            value: ChannelValue::CurrentPowerUsage(channel, Watt(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::TotalEnergyConsumption(channel) => PersistentDataPoint {
            value: ChannelValue::TotalEnergyConsumption(channel, KiloWattHours(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::SetPoint(channel) => {
            let v = match attributes.get("temperature").and_then(|v| v.as_f64()) {
                Some(f_value) => f_value,
                None => bail!("No temperature found in attributes or not a number"),
            };

            PersistentDataPoint {
                value: ChannelValue::SetPoint(channel, DegreeCelsius(v)),
                timestamp,
            }
        }
        HaChannel::HeatingDemand(channel) => PersistentDataPoint {
            value: ChannelValue::HeatingDemand(channel, Percent(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::PresenceFromLeakSensor(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(
                channel,
                if ha_value == "on" {
                    PresentState::Present
                } else {
                    PresentState::Absent
                },
            ),
            timestamp,
        },
        HaChannel::PresenceFromEsp(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(
                channel,
                if ha_value == "on" {
                    PresentState::Present
                } else {
                    PresentState::Absent
                },
            ),
            timestamp,
        },
        HaChannel::PresenceFromDeviceTracker(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(
                channel,
                if ha_value == "home" {
                    PresentState::Present
                } else {
                    PresentState::Absent
                },
            ),
            timestamp,
        },
    };

    Ok(dp)
}

mod parse {
    use std::collections::HashMap;

    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(tag = "event_type", content = "event_data")]
    pub enum HaEvent {
        #[serde(rename = "state_changed")]
        StateChanged {
            entity_id: String,
            new_state: StateChangedEvent,
        },

        #[serde(untagged)]
        Unknown(Value),
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct StateChangedEvent {
        pub entity_id: String,
        pub state: StateValue,
        pub last_changed: DateTime<Utc>,
        pub last_updated: DateTime<Utc>,
        pub attributes: HashMap<String, Value>,
    }

    #[derive(Debug)]
    pub enum StateValue {
        Available(String),
        Unavailable,
    }

    impl<'de> Deserialize<'de> for StateValue {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(deserializer)?;
            match value.as_str() {
                "unavailable" => Ok(StateValue::Unavailable),
                _ => Ok(StateValue::Available(value)),
            }
        }
    }

    impl Serialize for StateValue {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                StateValue::Available(v) => serializer.serialize_str(v),
                StateValue::Unavailable => serializer.serialize_str("unavailable"),
            }
        }
    }
}
