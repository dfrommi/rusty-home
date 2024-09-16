use std::env;

use parse::{HaEvent, StateValue};
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, warn};

use crate::adapter::{
    homeassistant::event::parse::StateChangedEvent, IncomingMessage, PersistentDataPoint,
};
use api::state::ChannelValue;
use support::unit::{DegreeCelsius, KiloWattHours, OpenedState, Percent, PowerState, Watt};

use super::{config::ha_incoming_event_config, HaChannel};

pub fn to_smart_home_event(json_payload: &str) -> Option<PersistentDataPoint> {
    let v = serde_json::from_str(json_payload);

    match v {
        Ok(HaEvent::StateChanged {
            entity_id,
            new_state,
            ..
        }) => {
            let ha_config = match ha_incoming_event_config(&entity_id as &str) {
                Some(entity_config) => entity_config,
                None => {
                    debug!("Skipped {}", entity_id);
                    return None;
                }
            };

            match new_state.state {
                StateValue::Available(state_value) => {
                    info!("Received supported event {}", entity_id);

                    let dp_result = to_persistent_data_point(
                        ha_config.channel,
                        &state_value,
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
        Ok(HaEvent::Unknown(_)) => {
            debug!("Received unsupported event");
            None
        }
        Err(err) => {
            error!("Error parsing json {:?}", err);
            None
        }
    }
}

pub async fn init(
    evt_tx: &Sender<IncomingMessage>,
    url: &str,
    token: &str,
) -> crate::error::Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/states", url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let events = response.json::<Vec<StateChangedEvent>>().await?;

    tracing::info!("{} init-events ready to send", events.len());

    for event in events.into_iter() {
        let entity_id = event.entity_id.clone();

        let msg = HaEvent::StateChanged {
            entity_id: entity_id.clone(),
            new_state: event,
        };

        let send_res = evt_tx
            .send(IncomingMessage::HomeAssistant {
                payload: json!(msg).to_string(),
            })
            .await;

        match send_res {
            Ok(_) => tracing::trace!("Sending init event for {}", entity_id),
            Err(e) => tracing::error!("Error sending init event for {}: {:?}", entity_id, e),
        }
    }

    Ok(())
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> crate::error::Result<PersistentDataPoint> {
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
