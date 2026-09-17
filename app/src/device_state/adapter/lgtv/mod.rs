use anyhow::{Context, bail};
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};

use crate::core::timeseries::DataPoint;
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{DeviceStateValue, EnergySaving, PowerAvailable};
use crate::t;

pub struct LgtvIncomingDataSource {
    base_topic: String,
    mqtt_receiver: MqttSubscription,
}

impl LgtvIncomingDataSource {
    pub async fn new(mqtt_client: &mut Mqtt, base_topic: &str) -> anyhow::Result<Self> {
        let mqtt_receiver = mqtt_client
            .subscribe_all(base_topic, &["state/picture/energySaving", "state/power/systemOn"])
            .await
            .context("Error subscribing to LG TV MQTT topics")?;

        Ok(Self {
            base_topic: base_topic.trim_matches('/').to_string(),
            mqtt_receiver,
        })
    }
}

impl IncomingDataSource for LgtvIncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let message = self.mqtt_receiver.recv().await?;

            match parse_lgtv_message(&self.base_topic, &message) {
                Ok(data) => return Some(vec![data]),
                Err(error) => {
                    tracing::error!("Error parsing LG TV MQTT message {:?}: {:?}", message, error);
                }
            }
        }
    }
}

fn parse_lgtv_message(base_topic: &str, message: &MqttInMessage) -> anyhow::Result<IncomingData> {
    let energy_saving_topic = format!("{base_topic}/state/picture/energySaving");
    let power_topic = format!("{base_topic}/state/power/systemOn");
    let timestamp = t!(now);

    if message.topic == energy_saving_topic {
        return Ok(DataPoint::new(
            DeviceStateValue::EnergySaving(EnergySaving::LivingRoomTv, message.payload != "off"),
            timestamp,
        )
        .into());
    }

    if message.topic == power_topic {
        let powered = match message.payload.as_str() {
            "true" => true,
            "false" => false,
            payload => bail!("Unexpected systemOn payload: {payload}"),
        };

        return Ok(DataPoint::new(
            DeviceStateValue::PowerAvailable(PowerAvailable::LivingRoomTv, powered),
            timestamp,
        )
        .into());
    }

    bail!("Unexpected LG TV topic: {}", message.topic)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn message(topic: &str, payload: &str) -> MqttInMessage {
        MqttInMessage {
            topic: topic.to_string(),
            payload: payload.to_string(),
        }
    }

    fn state_value(data: IncomingData) -> DeviceStateValue {
        match data {
            IncomingData::StateValue(data_point) => data_point.value,
            IncomingData::ItemAvailability(_) => panic!("expected state value"),
        }
    }

    #[test]
    fn parses_energy_saving_values() {
        assert_eq!(
            state_value(parse_lgtv_message("lgtv", &message("lgtv/state/picture/energySaving", "auto"),).unwrap()),
            DeviceStateValue::EnergySaving(EnergySaving::LivingRoomTv, true)
        );
        assert_eq!(
            state_value(parse_lgtv_message("lgtv", &message("lgtv/state/picture/energySaving", "off"),).unwrap()),
            DeviceStateValue::EnergySaving(EnergySaving::LivingRoomTv, false)
        );
    }

    #[test]
    fn treats_other_energy_saving_values_as_enabled() {
        assert_eq!(
            state_value(parse_lgtv_message("lgtv", &message("lgtv/state/picture/energySaving", "high"),).unwrap()),
            DeviceStateValue::EnergySaving(EnergySaving::LivingRoomTv, true)
        );
    }

    #[test]
    fn parses_system_power_values() {
        assert_eq!(
            state_value(parse_lgtv_message("lgtv", &message("lgtv/state/power/systemOn", "true"),).unwrap()),
            DeviceStateValue::PowerAvailable(PowerAvailable::LivingRoomTv, true)
        );
        assert_eq!(
            state_value(parse_lgtv_message("lgtv", &message("lgtv/state/power/systemOn", "false"),).unwrap()),
            DeviceStateValue::PowerAvailable(PowerAvailable::LivingRoomTv, false)
        );
    }

    #[test]
    fn rejects_invalid_system_power_values() {
        assert!(parse_lgtv_message("lgtv", &message("lgtv/state/power/systemOn", "on")).is_err());
    }
}
