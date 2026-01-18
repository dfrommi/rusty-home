mod config;

use crate::automation::Radiator;
use crate::core::DeviceConfig;
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, KiloWattHours, Percent, Watt};
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{DeviceAvailability, DeviceStateValue, Temperature};
use infrastructure::{Mqtt, MqttInMessage, MqttSubscription};

use crate::device_state::{CurrentPowerUsage, HeatingDemand, Opened, RelativeHumidity, TotalEnergyConsumption};

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption, KiloWattHours),
    SonoffThermostat(Radiator, HeatingDemand),
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
            Z2mChannel::SonoffThermostat(thermostat, demand) => {
                let payload: SonoffThermostatPayload = serde_json::from_str(&msg.payload)?;

                vec![
                    DataPoint::new(
                        DeviceStateValue::HeatingDemand(*demand, Percent(payload.valve_opening_degree)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        DeviceStateValue::Temperature(
                            Temperature::ThermostatOnDevice(*thermostat),
                            DegreeCelsius(payload.local_temperature),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::ContactSensor(opened) => {
                let payload: ContactSensor = serde_json::from_str(&msg.payload)?;
                vec![
                    DataPoint::new(DeviceStateValue::Opened(*opened, !payload.contact), payload.last_seen).into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::PowerPlug(power, energy, energy_offset) => {
                let payload: PowerPlug = serde_json::from_str(&msg.payload)?;
                vec![
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
                ]
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
struct SonoffThermostatPayload {
    system_mode: String,
    valve_opening_degree: f64,
    local_temperature: f64,
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
    last_seen: DateTime,
}
