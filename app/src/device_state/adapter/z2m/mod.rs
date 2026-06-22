mod config;

use std::collections::HashMap;

use crate::core::DeviceConfig;
use crate::core::domain::Radiator;
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, KiloWattHours, Percent, Watt};
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{
    DeviceAvailability, DeviceStateValue, HeatingDemandLimit, PowerAvailable, SetPoint, Temperature,
};
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};

use crate::device_state::{CurrentPowerUsage, Opened, RelativeHumidity, TotalEnergyConsumption};

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption, KiloWattHours, Option<PowerAvailable>),
    SonoffThermostat(Radiator),
}

pub struct Z2mIncomingDataSource {
    device_config: DeviceConfig<Z2mChannel>,
    mqtt_receiver: MqttSubscription,
}

impl Z2mIncomingDataSource {
    #[allow(clippy::expect_used)]
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> Self {
        let config = DeviceConfig::new(&config::default_z2m_state_config());
        let rx = mqtt_client
            .subscribe(event_topic, "#")
            .await
            .expect("Error subscribing to MQTT topic");

        Self {
            device_config: config,
            mqtt_receiver: rx,
        }
    }
}

impl IncomingDataSource for Z2mIncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let msg = self.mqtt_receiver.recv().await?;

            let Some(device_id) = z2m_device_id(&msg) else {
                continue;
            };

            let Some(channels) = self.device_config.get_optional(&device_id) else {
                continue;
            };

            if channels.is_empty() {
                continue;
            }

            tracing::debug!("Received Z2M event for devices {}: {:?}", device_id, channels);

            return Some(parse_configured_channels(&device_id, channels, &msg));
        }
    }
}

fn z2m_device_id(msg: &MqttInMessage) -> Option<String> {
    //Command topics end with /set and should be ignored. State not yet applied
    if msg.topic.ends_with("/set") {
        return None;
    }

    Some(msg.topic.clone())
}

fn parse_configured_channels(device_id: &str, channels: &[Z2mChannel], msg: &MqttInMessage) -> Vec<IncomingData> {
    let mut incoming_data = vec![];

    for channel in channels {
        match parse_z2m_channel(device_id, channel, &msg.payload) {
            Ok(events) => incoming_data.extend(events),
            Err(e) => {
                tracing::error!(
                    "Error parsing Z2M event for channel {:?} with payload {:?}: {:?}",
                    channel,
                    msg,
                    e
                );
            }
        }
    }

    incoming_data
}

fn parse_z2m_channel(device_id: &str, channel: &Z2mChannel, payload: &str) -> anyhow::Result<Vec<IncomingData>> {
    emit_debug_metrics(device_id, payload);

    match channel {
        Z2mChannel::ClimateSensor(temperature, humidity) => {
            parse_climate_sensor(device_id, *temperature, *humidity, payload)
        }
        Z2mChannel::ContactSensor(opened) => parse_contact_sensor(device_id, *opened, payload),
        Z2mChannel::PowerPlug(power, energy, energy_offset, power_available) => {
            parse_power_plug(device_id, *power, *energy, *energy_offset, *power_available, payload)
        }
        Z2mChannel::SonoffThermostat(thermostat) => parse_sonoff_thermostat(device_id, *thermostat, payload),
    }
}

fn parse_climate_sensor(
    device_id: &str,
    temperature: Temperature,
    humidity: RelativeHumidity,
    payload: &str,
) -> anyhow::Result<Vec<IncomingData>> {
    let payload: ClimateSensor = serde_json::from_str(payload)?;

    Ok(vec![
        DataPoint::new(
            DeviceStateValue::Temperature(temperature, DegreeCelsius(payload.temperature)),
            payload.last_seen,
        )
        .into(),
        DataPoint::new(
            DeviceStateValue::RelativeHumidity(humidity, Percent(payload.humidity)),
            payload.last_seen,
        )
        .into(),
        availability(device_id, payload.last_seen),
    ])
}

fn parse_contact_sensor(device_id: &str, opened: Opened, payload: &str) -> anyhow::Result<Vec<IncomingData>> {
    let payload: ContactSensor = serde_json::from_str(payload)?;

    Ok(vec![
        DataPoint::new(DeviceStateValue::Opened(opened, !payload.contact), payload.last_seen).into(),
        availability(device_id, payload.last_seen),
    ])
}

fn parse_power_plug(
    device_id: &str,
    power: CurrentPowerUsage,
    energy: TotalEnergyConsumption,
    energy_offset: KiloWattHours,
    power_available: Option<PowerAvailable>,
    payload: &str,
) -> anyhow::Result<Vec<IncomingData>> {
    let payload: PowerPlug = serde_json::from_str(payload)?;
    let mut items = vec![
        DataPoint::new(
            DeviceStateValue::CurrentPowerUsage(power, Watt(payload.current_power_w)),
            payload.last_seen,
        )
        .into(),
        DataPoint::new(
            DeviceStateValue::TotalEnergyConsumption(energy, KiloWattHours(payload.total_energy_kwh) + energy_offset),
            payload.last_seen,
        )
        .into(),
        availability(device_id, payload.last_seen),
    ];

    if let Some(power_available) = power_available {
        items.push(
            DataPoint::new(
                DeviceStateValue::PowerAvailable(power_available, payload.state == "ON"),
                payload.last_seen,
            )
            .into(),
        );
    }

    Ok(items)
}

fn parse_sonoff_thermostat(device_id: &str, thermostat: Radiator, payload: &str) -> anyhow::Result<Vec<IncomingData>> {
    let payload: SonoffThermostatPayload = serde_json::from_str(payload)?;

    let mut result = vec![
        DataPoint::new(
            DeviceStateValue::Temperature(
                Temperature::ThermostatOnDevice(thermostat),
                DegreeCelsius(payload.local_temperature),
            ),
            payload.last_seen,
        )
        .into(),
        DataPoint::new(
            DeviceStateValue::Temperature(
                Temperature::ThermostatExternalInput(thermostat),
                DegreeCelsius(payload.external_temperature_input),
            ),
            payload.last_seen,
        )
        .into(),
        availability(device_id, payload.last_seen),
    ];

    let (setpoint_lower, setpoint_upper, demand_lower, demand_upper) = match thermostat {
        Radiator::LivingRoomBig => (
            SetPoint::LivingRoomBigLower,
            SetPoint::LivingRoomBig,
            HeatingDemandLimit::LivingRoomBigLower,
            HeatingDemandLimit::LivingRoomBigUpper,
        ),
        Radiator::LivingRoomSmall => (
            SetPoint::LivingRoomSmallLower,
            SetPoint::LivingRoomSmall,
            HeatingDemandLimit::LivingRoomSmallLower,
            HeatingDemandLimit::LivingRoomSmallUpper,
        ),
        Radiator::Bedroom => (
            SetPoint::BedroomLower,
            SetPoint::Bedroom,
            HeatingDemandLimit::BedroomLower,
            HeatingDemandLimit::BedroomUpper,
        ),
        Radiator::Kitchen => (
            SetPoint::KitchenLower,
            SetPoint::Kitchen,
            HeatingDemandLimit::KitchenLower,
            HeatingDemandLimit::KitchenUpper,
        ),
        Radiator::RoomOfRequirements => (
            SetPoint::RoomOfRequirementsLower,
            SetPoint::RoomOfRequirements,
            HeatingDemandLimit::RoomOfRequirementsLower,
            HeatingDemandLimit::RoomOfRequirementsUpper,
        ),
        Radiator::Bathroom => (
            SetPoint::BathroomLower,
            SetPoint::Bathroom,
            HeatingDemandLimit::BathroomLower,
            HeatingDemandLimit::BathroomUpper,
        ),
    };

    let is_off = payload.system_mode.as_deref() == Some("off");
    let (setpoint_lower_value, setpoint_upper_value) = if is_off {
        (DegreeCelsius(0.0), DegreeCelsius(0.0))
    } else {
        (
            DegreeCelsius(payload.occupied_heating_setpoint + payload.temperature_accuracy),
            DegreeCelsius(payload.occupied_heating_setpoint),
        )
    };

    result.push(
        DataPoint::new(
            DeviceStateValue::SetPoint(setpoint_upper, setpoint_upper_value),
            payload.last_seen,
        )
        .into(),
    );

    result.push(
        DataPoint::new(
            DeviceStateValue::SetPoint(setpoint_lower, setpoint_lower_value),
            payload.last_seen,
        )
        .into(),
    );

    result.push(
        DataPoint::new(
            DeviceStateValue::HeatingDemandLimit(demand_upper, Percent(payload.valve_opening_degree).clamp()),
            payload.last_seen,
        )
        .into(),
    );

    result.push(
        DataPoint::new(
            DeviceStateValue::HeatingDemandLimit(demand_lower, Percent(100.0 - payload.valve_closing_degree).clamp()),
            payload.last_seen,
        )
        .into(),
    );

    //Current demand not exposed directly at running state is not reliable and no other
    //way of reading the current demand exists.

    Ok(result)
}

fn availability(friendly_name: &str, last_seen: DateTime) -> IncomingData {
    DeviceAvailability {
        source: "Z2M".to_string(),
        device_id: friendly_name.to_string(),
        last_seen,
        marked_offline: false,
    }
    .into()
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ClimateSensor {
    temperature: f64,
    humidity: f64,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ContactSensor {
    contact: bool,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PowerPlug {
    #[serde(rename = "power")]
    current_power_w: f64,
    #[serde(rename = "energy")]
    total_energy_kwh: f64,
    state: String,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SonoffThermostatPayload {
    system_mode: Option<String>,
    valve_opening_degree: f64,
    valve_closing_degree: f64,
    occupied_heating_setpoint: f64,
    temperature_accuracy: f64, //negative
    local_temperature: f64,
    external_temperature_input: f64,
    last_seen: DateTime,
}

fn emit_debug_metrics(device_id: &str, payload: &str) {
    let parsed: HashMap<String, serde_json::Value> = match serde_json::from_str(payload) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Error parsing Sonoff thermostat payload for debug metrics: {:?}", e);
            return;
        }
    };

    const METRIC_NAME: &str = "z2m_state";

    for (key, value) in parsed {
        let f_value = if let Some(num) = value.as_number().and_then(|n| n.as_f64()) {
            Some(num)
        } else if let Some(b) = value.as_bool() {
            Some(if b { 1.0 } else { 0.0 })
        } else if let Some(s) = value.as_str() {
            match s.to_lowercase().as_str() {
                "off" | "unlock" | "internal" => Some(0.0),
                "on" | "heat" | "lock" => Some(1.0),
                "auto" => Some(2.0),
                "timer" => Some(3.0),
                "boost" => Some(5.0),
                _ if s.starts_with("external") => Some(1.0),
                _ if key == "last_seen" => DateTime::from_iso(s).ok().map(|dt| dt.elapsed().as_minutes_f64()),
                _ => None,
            }
        } else if value.is_null() {
            Some(-99.0)
        } else {
            None
        };

        if let Some(f_value) = f_value {
            crate::observability::system_metric_set(
                METRIC_NAME,
                f_value,
                &[("item", key.as_str()), ("device_id", device_id)],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_values(items: Vec<IncomingData>) -> Vec<DeviceStateValue> {
        items
            .into_iter()
            .filter_map(|item| match item {
                IncomingData::StateValue(data_point) => Some(data_point.value),
                IncomingData::ItemAvailability(_) => None,
            })
            .collect()
    }

    #[test]
    fn ignores_command_topics() {
        let msg = MqttInMessage {
            topic: "living_room/sensor/set".to_string(),
            payload: "{}".to_string(),
        };

        assert!(z2m_device_id(&msg).is_none());
    }

    #[test]
    fn parse_error_in_one_channel_does_not_block_other_channels() {
        let msg = MqttInMessage {
            topic: "power/plug".to_string(),
            payload: r#"{
                "power": 42.0,
                "energy": 10.5,
                "state": "ON",
                "last_seen": "2025-01-01T00:00:00+00:00"
            }"#
            .to_string(),
        };
        let channels = vec![
            Z2mChannel::ContactSensor(Opened::KitchenWindow),
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::Kettle,
                TotalEnergyConsumption::Kettle,
                KiloWattHours(1.0),
                Some(PowerAvailable::InfraredHeater),
            ),
        ];

        let values = state_values(parse_configured_channels("power/plug", &channels, &msg));

        assert_eq!(
            values,
            vec![
                DeviceStateValue::CurrentPowerUsage(CurrentPowerUsage::Kettle, Watt(42.0)),
                DeviceStateValue::TotalEnergyConsumption(TotalEnergyConsumption::Kettle, KiloWattHours(11.5)),
                DeviceStateValue::PowerAvailable(PowerAvailable::InfraredHeater, true),
            ]
        );
    }
}
