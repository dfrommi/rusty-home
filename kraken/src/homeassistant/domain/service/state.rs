use std::collections::HashMap;

use anyhow::bail;
use api::state::ChannelValue;
use support::{
    DataPoint,
    time::DateTime,
    unit::{DegreeCelsius, Percent},
};
use tokio::sync::mpsc;

use crate::{
    core::{IncomingData, IncomingDataProcessor, ItemAvailability},
    homeassistant::domain::{
        GetAllEntityStatesPort, HaChannel, ListenToStateChangesPort, StateChangedEvent, StateValue,
    },
};

pub struct HaIncomingDataProcessor<C, L> {
    client: C,
    listener: L,
    config: HashMap<String, Vec<HaChannel>>,
}

impl<C: GetAllEntityStatesPort, L: ListenToStateChangesPort> HaIncomingDataProcessor<C, L> {
    pub fn new(client: C, listener: L, config: &[(&str, HaChannel)]) -> Self {
        let mut m: HashMap<String, Vec<HaChannel>> = HashMap::new();
        for (id, channel) in config {
            let id = id.to_string();
            m.entry(id).or_default().push(channel.clone());
        }

        Self {
            client,
            listener,
            config: m,
        }
    }
}

impl<C: GetAllEntityStatesPort, L: ListenToStateChangesPort> IncomingDataProcessor
    for HaIncomingDataProcessor<C, L>
{
    async fn process(&mut self, sender: mpsc::Sender<IncomingData>) -> anyhow::Result<()> {
        tracing::info!("Requesting current state from HA");

        let current_dps: Vec<IncomingData> = self
            .client
            .get_current_state()
            .await?
            .iter()
            .flat_map(|e| to_smart_home_events(e, &self.config))
            .collect();

        for dp in current_dps {
            sender.send(dp).await?;
        }

        tracing::info!("Waiting for HA state changes");

        loop {
            match ListenToStateChangesPort::recv(&mut self.listener).await {
                Ok(event) => {
                    let dps = to_smart_home_events(&event, &self.config);

                    for dp in dps {
                        sender.send(dp).await?;
                    }
                }

                //json parsing error
                Err(e) => tracing::error!("Error parsing MQTT message: {}", e),
            }
        }
    }
}

fn to_smart_home_events(
    new_state: &StateChangedEvent,
    config: &HashMap<String, Vec<HaChannel>>,
) -> Vec<IncomingData> {
    let entity_id: &str = &new_state.entity_id;

    let ha_channels = config.get(entity_id as &str);

    if ha_channels.is_none() {
        tracing::trace!("Skipped {}", entity_id);
        return vec![];
    }

    let ha_channels = ha_channels.unwrap();

    let mut result = match &new_state.state {
        StateValue::Available(state_value) => {
            tracing::info!("Received supported event {}", entity_id);

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
                        Ok(dp) => dp,
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
            tracing::warn!("Value of {} is not available", entity_id);
            vec![]
        }
    };

    result.push(to_item_availability(new_state));

    result
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    attributes: &HashMap<String, serde_json::Value>,
    timestamp: DateTime,
) -> anyhow::Result<Option<IncomingData>> {
    let dp: Option<IncomingData> = match channel {
        HaChannel::Temperature(channel) => Some(
            DataPoint::new(
                ChannelValue::Temperature(channel, DegreeCelsius(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::RelativeHumidity(channel) => Some(
            DataPoint::new(
                ChannelValue::RelativeHumidity(channel, Percent(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::Powered(channel) => {
            Some(DataPoint::new(ChannelValue::Powered(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::SetPoint(channel) => {
            let v = match (
                ha_value,
                attributes.get("temperature").and_then(|v| v.as_f64()),
            ) {
                ("off", _) => 0.0,
                (_, Some(f_value)) => f_value,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            Some(
                DataPoint::new(
                    ChannelValue::SetPoint(channel, DegreeCelsius::from(v)),
                    timestamp,
                )
                .into(),
            )
        }
        HaChannel::HeatingDemand(channel) => Some(
            DataPoint::new(
                ChannelValue::HeatingDemand(channel, Percent(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::ClimateAutoMode(channel) => Some(
            DataPoint::new(
                ChannelValue::ExternalAutoControl(channel, ha_value == "auto"),
                timestamp,
            )
            .into(),
        ),
        HaChannel::PresenceFromEsp(channel) => Some(
            DataPoint::new(ChannelValue::Presence(channel, ha_value == "on"), timestamp).into(),
        ),
        HaChannel::PresenceFromDeviceTracker(channel) => Some(
            DataPoint::new(
                ChannelValue::Presence(channel, ha_value == "home"),
                timestamp,
            )
            .into(),
        ),
        HaChannel::WindcalmFanSpeed(channel) => {
            let v = match (
                ha_value,
                attributes.get("percentage").and_then(|v| v.as_f64()),
            ) {
                ("off", _) => 0.0,
                (_, Some(f_value)) => f_value,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            Some(
                DataPoint::new(ChannelValue::FanSpeed(channel, Percent::from(v)), timestamp).into(),
            )
        }
    };

    Ok(dp)
}

fn to_item_availability(new_state: &StateChangedEvent) -> IncomingData {
    let entity_id: &str = &new_state.entity_id;

    ItemAvailability {
        source: "HA".to_string(),
        item: entity_id.to_string(),
        last_seen: new_state.last_updated,
        marked_offline: matches!(new_state.state, StateValue::Unavailable),
    }
    .into()
}
