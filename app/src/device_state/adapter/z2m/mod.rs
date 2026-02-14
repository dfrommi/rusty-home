mod config;

use std::collections::HashMap;

use crate::automation::Radiator;
use crate::core::DeviceConfig;
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
    base_topic: String,
    device_config: DeviceConfig<Z2mChannel>,
    mqtt_receiver: MqttSubscription,
}

impl Z2mIncomingDataSource {
    pub async fn new(mqtt_client: &mut Mqtt, event_topic: &str) -> Self {
        let config = DeviceConfig::new(&config::default_z2m_state_config());
        let rx = mqtt_client
            .subscribe(format!("{}/#", event_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        Self {
            base_topic: event_topic.trim_matches('/').to_owned(),
            device_config: config,
            mqtt_receiver: rx,
        }
    }
}

impl IncomingDataSource<MqttInMessage, Z2mChannel> for Z2mIncomingDataSource {
    fn ds_name(&self) -> &str {
        "Z2M"
    }

    async fn recv(&mut self) -> Option<MqttInMessage> {
        self.mqtt_receiver.recv().await
    }

    fn device_id(&self, msg: &MqttInMessage) -> Option<String> {
        let topic = &msg.topic;

        //Command topics end with /set and should be ignored. State not yet applied
        if topic.ends_with("/set") {
            return None;
        }

        topic
            .strip_prefix(&self.base_topic)
            .map(|topic| topic.trim_matches('/').to_owned())
    }

    fn get_channels(&self, device_id: &str) -> &[Z2mChannel] {
        self.device_config.get(device_id)
    }

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &Z2mChannel,
        msg: &MqttInMessage,
    ) -> anyhow::Result<Vec<IncomingData>> {
        emit_debug_metrics(device_id, &msg.payload);

        let result: Vec<IncomingData> = match channel {
            Z2mChannel::ClimateSensor(t, h) => {
                let payload: ClimateSensor = serde_json::from_str(&msg.payload)?;

                vec![
                    DataPoint::new(
                        DeviceStateValue::Temperature(*t, DegreeCelsius(payload.temperature)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        DeviceStateValue::RelativeHumidity(*h, Percent(payload.humidity)),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            //Sonoff thermostats
            Z2mChannel::SonoffThermostat(thermostat) => {
                let payload: SonoffThermostatPayload = serde_json::from_str(&msg.payload)?;

                let mut result = vec![
                    DataPoint::new(
                        DeviceStateValue::Temperature(
                            Temperature::ThermostatOnDevice(*thermostat),
                            DegreeCelsius(payload.local_temperature),
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

                result.push(
                    DataPoint::new(
                        DeviceStateValue::SetPoint(setpoint_upper, DegreeCelsius(payload.occupied_heating_setpoint)),
                        payload.last_seen,
                    )
                    .into(),
                );

                result.push(
                    DataPoint::new(
                        DeviceStateValue::SetPoint(
                            setpoint_lower,
                            DegreeCelsius(payload.occupied_heating_setpoint + payload.temperature_accuracy),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                );

                result.push(
                    DataPoint::new(
                        DeviceStateValue::HeatingDemandLimit(
                            demand_upper,
                            Percent(payload.valve_opening_degree).clamp(),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                );

                result.push(
                    DataPoint::new(
                        DeviceStateValue::HeatingDemandLimit(
                            demand_lower,
                            Percent(100.0 - payload.valve_closing_degree).clamp(),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                );

                //Current demand not exposed directly at running state is not reliable and no other
                //way of reading the current demand exists.

                result
            }

            Z2mChannel::ContactSensor(opened) => {
                let payload: ContactSensor = serde_json::from_str(&msg.payload)?;
                vec![
                    DataPoint::new(DeviceStateValue::Opened(*opened, !payload.contact), payload.last_seen).into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::PowerPlug(power, energy, energy_offset, power_available) => {
                let payload: PowerPlug = serde_json::from_str(&msg.payload)?;
                let mut items = vec![
                    DataPoint::new(
                        DeviceStateValue::CurrentPowerUsage(*power, Watt(payload.current_power_w)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        DeviceStateValue::TotalEnergyConsumption(
                            *energy,
                            KiloWattHours(payload.total_energy_kwh) + *energy_offset,
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ];

                if let Some(power_available) = power_available {
                    items.push(
                        DataPoint::new(
                            DeviceStateValue::PowerAvailable(*power_available, payload.state == "ON"),
                            payload.last_seen,
                        )
                        .into(),
                    );
                }

                items
            }
        };

        Ok(result)
    }
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
    valve_opening_degree: f64,
    valve_closing_degree: f64,
    occupied_heating_setpoint: f64,
    temperature_accuracy: f64, //negative
    local_temperature: f64,
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
