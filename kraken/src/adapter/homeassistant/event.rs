use std::collections::HashMap;

use parse::{HaEvent, StateValue};
use serde_json::Value;
use support::mqtt::MqttInMessage;
use support::time::DateTime;
use support::unit::*;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::adapter::config::ha_incoming_event_channel;
use crate::adapter::StateCollector;
use crate::adapter::{homeassistant::event::parse::StateChangedEvent, PersistentDataPoint};
use anyhow::{bail, Result};
use api::state::ChannelValue;

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
                Ok(event) => {
                    for dp in to_smart_home_events(&event) {
                        if let Err(e) = dp_tx.send(dp).await {
                            tracing::error!(
                                "Error sending data-point to channel for processing: {}",
                                e
                            );
                        }
                    }
                }

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
            for dp in to_smart_home_events(&event) {
                dp_tx.send(dp).await?;
            }
        }

        Ok(())
    }
}

fn to_smart_home_events(event: &HaEvent) -> Vec<PersistentDataPoint> {
    match event {
        HaEvent::StateChanged {
            entity_id,
            new_state,
            ..
        } => {
            let ha_channels = ha_incoming_event_channel(entity_id as &str);

            if ha_channels.is_empty() {
                tracing::trace!("Skipped {}", entity_id);
                return vec![];
            }

            match &new_state.state {
                StateValue::Available(state_value) => {
                    info!("Received supported event {}", entity_id);

                    ha_channels
                        .iter()
                        .filter_map(|ha_channel| {
                            let dp_result = to_persistent_data_point(
                                ha_channel.clone(),
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
                        })
                        .collect()
                }
                _ => {
                    warn!("Value of {} is not available", entity_id);
                    vec![]
                }
            }
        }
        HaEvent::Unknown(_) => {
            debug!("Received unsupported event");
            vec![]
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
    timestamp: DateTime,
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
            value: ChannelValue::Opened(channel, ha_value == "on"),
            timestamp,
        },
        HaChannel::Powered(channel) => PersistentDataPoint {
            value: ChannelValue::Powered(channel, ha_value == "on"),
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
            let v = match (
                ha_value,
                attributes.get("temperature").and_then(|v| v.as_f64()),
            ) {
                ("off", _) => 0.0,
                (_, Some(f_value)) => f_value,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            PersistentDataPoint {
                value: ChannelValue::SetPoint(channel, DegreeCelsius::from(v)),
                timestamp,
            }
        }
        HaChannel::HeatingDemand(channel) => PersistentDataPoint {
            value: ChannelValue::HeatingDemand(channel, Percent(ha_value.parse()?)),
            timestamp,
        },
        HaChannel::ClimateAutoMode(channel) => PersistentDataPoint {
            value: ChannelValue::ExternalAutoControl(channel, ha_value == "auto"),
            timestamp,
        },
        HaChannel::PresenceFromLeakSensor(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(channel, ha_value == "on"),
            timestamp,
        },
        HaChannel::PresenceFromEsp(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(channel, ha_value == "on"),
            timestamp,
        },
        HaChannel::PresenceFromDeviceTracker(channel) => PersistentDataPoint {
            value: ChannelValue::Presence(channel, ha_value == "home"),
            timestamp,
        },
    };

    Ok(dp)
}

mod parse {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;
    use support::time::DateTime;

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
        pub last_changed: DateTime,
        pub last_updated: DateTime,
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
