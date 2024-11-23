use std::collections::HashMap;

use anyhow::bail;
use api::state::ChannelValue;
use support::{
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent, Watt},
    DataPoint,
};

use crate::{
    core::StateCollector,
    homeassistant::domain::{
        GetAllEntityStatesPort, HaChannel, ListenToStateChangesPort, StateChangedEvent, StateValue,
    },
};

pub struct HaStateCollector<C, L> {
    client: C,
    listener: L,
    config: HashMap<String, Vec<HaChannel>>,
    pending_events: Vec<DataPoint<ChannelValue>>,
}

impl<C: GetAllEntityStatesPort, L: ListenToStateChangesPort> HaStateCollector<C, L> {
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
            pending_events: vec![],
        }
    }
}

impl<C: GetAllEntityStatesPort, L: ListenToStateChangesPort> StateCollector
    for HaStateCollector<C, L>
{
    async fn get_current_state(&self) -> anyhow::Result<Vec<DataPoint<ChannelValue>>> {
        let events = self.client.get_current_state().await?;

        let dps = events
            .iter()
            .flat_map(|e| to_smart_home_events(e, &self.config))
            .collect();
        Ok(dps)
    }

    async fn recv(&mut self) -> anyhow::Result<DataPoint<ChannelValue>> {
        if !self.pending_events.is_empty() {
            return Ok(self.pending_events.remove(0));
        }

        loop {
            match ListenToStateChangesPort::recv(&mut self.listener).await {
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

fn to_smart_home_events(
    new_state: &StateChangedEvent,
    config: &HashMap<String, Vec<HaChannel>>,
) -> Vec<DataPoint<ChannelValue>> {
    let entity_id: &str = &new_state.entity_id;

    let ha_channels = config.get(entity_id as &str);

    if ha_channels.is_none() {
        tracing::trace!("Skipped {}", entity_id);
        return vec![];
    }

    let ha_channels = ha_channels.unwrap();

    match &new_state.state {
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
            tracing::warn!("Value of {} is not available", entity_id);
            vec![]
        }
    }
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    attributes: &HashMap<String, serde_json::Value>,
    timestamp: DateTime,
) -> anyhow::Result<DataPoint<ChannelValue>> {
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
