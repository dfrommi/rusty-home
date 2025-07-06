use std::collections::HashMap;

use crate::{
    core::{time::DateTime, timeseries::DataPoint},
    home::state::{ChannelValue, FanAirflow},
};
use anyhow::bail;
use crate::core::unit::{DegreeCelsius, Percent};

use super::{HaChannel, HaHttpClient, HaMqttClient, StateChangedEvent, StateValue};
use crate::core::{DeviceConfig, IncomingData, IncomingDataSource, ItemAvailability};

pub struct HaIncomingDataSource {
    client: HaHttpClient,
    listener: HaMqttClient,
    config: DeviceConfig<HaChannel>,
    initial_load: Option<Vec<StateChangedEvent>>,
}

impl HaIncomingDataSource {
    pub fn new(
        client: HaHttpClient,
        listener: HaMqttClient,
        config: DeviceConfig<HaChannel>,
    ) -> Self {
        Self {
            client,
            listener,
            config,
            initial_load: None,
        }
    }
}

impl IncomingDataSource<StateChangedEvent, HaChannel> for HaIncomingDataSource {
    async fn recv(&mut self) -> Option<StateChangedEvent> {
        if self.initial_load.is_none() {
            self.initial_load = match self.client.get_current_state().await {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::error!("Error loading initial state for HA: {:?}", e);
                    Some(vec![])
                }
            };
        }

        match &mut self.initial_load {
            Some(data) if !data.is_empty() => data.pop(),
            _ => self.listener.recv().await,
        }
    }

    fn device_id(&self, msg: &StateChangedEvent) -> Option<String> {
        Some(msg.entity_id.clone())
    }

    fn get_channels(&self, device_id: &str) -> &[HaChannel] {
        self.config.get(device_id)
    }

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &HaChannel,
        msg: &StateChangedEvent,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let mut result = match &msg.state {
            StateValue::Available(state_value) => {
                tracing::info!("Received supported event {}", device_id);

                let dp_result = to_persistent_data_point(
                    channel.clone(),
                    state_value,
                    &msg.attributes,
                    msg.last_changed,
                );

                match dp_result {
                    Ok(Some(dp)) => vec![dp],
                    Ok(None) => vec![],
                    Err(e) => {
                        tracing::error!(
                            "Error processing homeassistant event of {}: {:?}",
                            device_id,
                            e
                        );
                        vec![]
                    }
                }
            }
            _ => {
                tracing::warn!("Value of {} is not available", device_id);
                vec![]
            }
        };

        result.push(to_item_availability(msg));

        Ok(result)
    }
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
            //Fan-Speed updates are extremely unreliable at the moment. Only use Off as a reset
            //trigger, otherwise assume command worked an directly set state from command
            //processing, assuming everything is done via the smart home
            /*
            last_seent fan_speed = match attributes.get("percentage").and_then(|v| v.as_f64()) {
                Some(1.0) => FanSpeed::Silent,
                Some(f_value) if f_value <= 20.0 => FanSpeed::Low,
                Some(f_value) if f_value <= 40.0 => FanSpeed::Medium,
                Some(f_value) if f_value <= 60.0 => FanSpeed::High,
                Some(_) => FanSpeed::Turbo,
                _ => bail!("No temperature found in attributes or not a number"),
            };

            let airflow = if ha_value == "off" {
                FanAirflow::Off
            } else if attributes.get("direction").and_then(|v| v.as_str()) == Some("reverse") {
                FanAirflow::Reverse(fan_speed)
            } else {
                FanAirflow::Forward(fan_speed)
            };

            Some(DataPoint::new(ChannelValue::FanActivity(channel, airflow), timestamp).into())
            */

            if ha_value == "off" {
                Some(
                    DataPoint::new(
                        ChannelValue::FanActivity(channel, FanAirflow::Off),
                        timestamp,
                    )
                    .into(),
                )
            } else {
                None
            }
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
