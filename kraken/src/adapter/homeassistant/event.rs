use std::collections::HashMap;

use parse::{HaEvent, StateValue};
use serde_json::Value;
use support::mqtt::MqttInMessage;
use support::time::DateTime;
use support::{unit::*, DataPoint};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::adapter::homeassistant::event::parse::StateChangedEvent;
use crate::port::StateCollector;
use anyhow::{bail, Result};
use api::state::ChannelValue;

use super::{HaChannel, HaRestClient};

//TODO treat conversions via Into traits, maybe introduce intermediate structs

pub struct HaStateCollector {
    api: HaRestClient,
    event_rx: mpsc::Receiver<MqttInMessage>,
    config: HashMap<String, Vec<HaChannel>>,
    pending_events: Vec<DataPoint<ChannelValue>>,
}

impl HaStateCollector {
    pub fn new(
        client: HaRestClient,
        event_rx: mpsc::Receiver<MqttInMessage>,
        config: &[(&str, HaChannel)],
    ) -> Self {
        let mut m: HashMap<String, Vec<HaChannel>> = HashMap::new();
        for (id, channel) in config {
            let id = id.to_string();
            m.entry(id).or_default().push(channel.clone());
        }

        Self {
            api: client,
            event_rx,
            config: m,
            pending_events: vec![],
        }
    }
}

impl StateCollector for HaStateCollector {
    async fn get_current_state(&self) -> anyhow::Result<Vec<DataPoint<ChannelValue>>> {
        let mut result: Vec<DataPoint<ChannelValue>> = vec![];

        for event in get_current_states(&self.api).await? {
            for dp in to_smart_home_events(&event, &self.config) {
                result.push(dp);
            }
        }

        Ok(result)
    }

    async fn recv(&mut self) -> anyhow::Result<DataPoint<ChannelValue>> {
        if !self.pending_events.is_empty() {
            return Ok(self.pending_events.remove(0));
        }

        loop {
            if let Some(msg) = self.event_rx.recv().await {
                match serde_json::from_str(&msg.payload) {
                    Ok(event) => {
                        let mut dps = to_smart_home_events(&event, &self.config);

                        if !dps.is_empty() {
                            let dp = dps.remove(0);
                            self.pending_events.extend(dps);
                            return Ok(dp);
                        }
                    }

                    //json parsing error
                    Err(e) => tracing::error!("Error parsing MQTT message: {}", e),
                }
            }
        }
    }
}

fn to_smart_home_events(
    event: &HaEvent,
    config: &HashMap<String, Vec<HaChannel>>,
) -> Vec<DataPoint<ChannelValue>> {
    match event {
        HaEvent::StateChanged {
            entity_id,
            new_state,
            ..
        } => {
            let ha_channels = config.get(entity_id as &str);

            if ha_channels.is_none() {
                tracing::trace!("Skipped {}", entity_id);
                return vec![];
            }

            let ha_channels = ha_channels.unwrap();

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

async fn get_current_states(api: &HaRestClient) -> Result<Vec<HaEvent>> {
    let response = api.get_all_states().await?;

    let events = response
        .into_iter()
        .map(serde_json::from_value::<StateChangedEvent>)
        .collect::<Result<Vec<_>, _>>()?;

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
) -> Result<DataPoint<ChannelValue>> {
    let dp = match channel {
        HaChannel::Temperature(channel) => DataPoint::new(
            ChannelValue::Temperature(channel, DegreeCelsius(ha_value.parse()?)),
            timestamp,
        ),
        HaChannel::RelativeHumidity(channel) => DataPoint::new(
            ChannelValue::RelativeHumidity(channel, Percent(ha_value.parse()?)),
            timestamp,
        ),
        HaChannel::Opened(channel) => {
            DataPoint::new(ChannelValue::Opened(channel, ha_value == "on"), timestamp)
        }
        HaChannel::Powered(channel) => {
            DataPoint::new(ChannelValue::Powered(channel, ha_value == "on"), timestamp)
        }
        HaChannel::CurrentPowerUsage(channel) => DataPoint::new(
            ChannelValue::CurrentPowerUsage(channel, Watt(ha_value.parse()?)),
            timestamp,
        ),
        HaChannel::TotalEnergyConsumption(channel) => DataPoint::new(
            ChannelValue::TotalEnergyConsumption(channel, KiloWattHours(ha_value.parse()?)),
            timestamp,
        ),
        HaChannel::SetPoint(channel) => {
            let v = match (
                ha_value,
                attributes.get("temperature").and_then(|v| v.as_f64()),
            ) {
                ("off", _) => 0.0,
                (_, Some(f_value)) => f_value,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            DataPoint::new(
                ChannelValue::SetPoint(channel, DegreeCelsius::from(v)),
                timestamp,
            )
        }
        HaChannel::HeatingDemand(channel) => DataPoint::new(
            ChannelValue::HeatingDemand(channel, Percent(ha_value.parse()?)),
            timestamp,
        ),
        HaChannel::ClimateAutoMode(channel) => DataPoint::new(
            ChannelValue::ExternalAutoControl(channel, ha_value == "auto"),
            timestamp,
        ),
        HaChannel::PresenceFromLeakSensor(channel) => {
            DataPoint::new(ChannelValue::Presence(channel, ha_value == "on"), timestamp)
        }
        HaChannel::PresenceFromEsp(channel) => {
            DataPoint::new(ChannelValue::Presence(channel, ha_value == "on"), timestamp)
        }
        HaChannel::PresenceFromDeviceTracker(channel) => DataPoint::new(
            ChannelValue::Presence(channel, ha_value == "home"),
            timestamp,
        ),
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
