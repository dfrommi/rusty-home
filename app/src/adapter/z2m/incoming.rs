use crate::adapter::incoming::{IncomingData, IncomingDataSource};
use crate::adapter::z2m::outgoing::Z2mCommandExecutor;
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::core::unit::{DegreeCelsius, KiloWattHours, Percent, Watt};
use crate::home::availability::ItemAvailability;
use crate::home::state::{PersistentHomeStateValue, Temperature};
use crate::home::trigger::{ButtonPress, Remote, RemoteTarget, UserTrigger};
use crate::t;
use infrastructure::MqttInMessage;
use tokio::sync::mpsc;

use crate::core::DeviceConfig;

use super::Z2mChannel;

pub struct Z2mIncomingDataSource {
    base_topic: String,
    device_config: DeviceConfig<Z2mChannel>,
    mqtt_receiver: mpsc::Receiver<MqttInMessage>,
    executor: Z2mCommandExecutor,
}

impl Z2mIncomingDataSource {
    pub fn new(
        base_topic: String,
        config: DeviceConfig<Z2mChannel>,
        mqtt_rx: mpsc::Receiver<MqttInMessage>,
        executor: Z2mCommandExecutor,
    ) -> Self {
        Self {
            base_topic: base_topic.trim_matches('/').to_owned(),
            device_config: config,
            mqtt_receiver: mqtt_rx,
            executor,
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
                        PersistentHomeStateValue::Temperature(t.clone(), DegreeCelsius(payload.temperature)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::RelativeHumidity(h.clone(), Percent(payload.humidity)),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::Thermostat(thermostat, set_point, demand, opened, group) => {
                let payload: Thermostat = serde_json::from_str(&msg.payload)?;

                if let Some(group) = group {
                    let mut group = group.lock().await;
                    group.register_load(device_id, DataPoint::new(payload.load_estimate, payload.last_seen));
                    if let Err(e) = group.send_if_needed(&self.executor).await {
                        tracing::error!("Error sending load estimate to thermostat group: {:?}", e);
                    }
                }

                vec![
                    DataPoint::new(
                        PersistentHomeStateValue::SetPoint(
                            set_point.clone(),
                            DegreeCelsius(if payload.window_open_external {
                                //internal window-open temperature is 8-12 degrees
                                10.0
                            } else {
                                payload.occupied_heating_setpoint
                            }),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::HeatingDemand(
                            demand.clone(),
                            Percent(if payload.pi_heating_demand > 1.0 {
                                payload.pi_heating_demand
                            } else {
                                //sometimes reports 1% when closed
                                0.0
                            }),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::Opened(opened.clone(), payload.window_open_external),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::Temperature(
                            Temperature::ThermostatOnDevice(thermostat.clone()),
                            DegreeCelsius(payload.local_temperature),
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::Temperature(
                            Temperature::ThermostatExternal(thermostat.clone()),
                            DegreeCelsius(payload.external_measured_room_sensor / 100.0),
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
                    DataPoint::new(
                        PersistentHomeStateValue::Opened(opened.clone(), !payload.contact),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::PowerPlug(power, energy, energy_offset) => {
                let payload: PowerPlug = serde_json::from_str(&msg.payload)?;
                vec![
                    DataPoint::new(
                        PersistentHomeStateValue::CurrentPowerUsage(power.clone(), Watt(payload.current_power_w)),
                        payload.last_seen,
                    )
                    .into(),
                    DataPoint::new(
                        PersistentHomeStateValue::TotalEnergyConsumption(
                            energy.clone(),
                            KiloWattHours(payload.total_energy_kwh) + *energy_offset,
                        ),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::PresenceFromLeakSensor(presence) => {
                let payload: WaterLeakSensor = serde_json::from_str(&msg.payload)?;
                vec![
                    DataPoint::new(
                        PersistentHomeStateValue::Presence(presence.clone(), payload.water_leak),
                        payload.last_seen,
                    )
                    .into(),
                    availability(device_id, payload.last_seen),
                ]
            }

            Z2mChannel::RemoteClick(target) => {
                let payload: RemoteControl = serde_json::from_str(&msg.payload)?;
                let mut events = vec![availability(device_id, payload.last_seen)];

                let button_press = match payload.action.as_deref() {
                    Some("on") => Some(ButtonPress::TopSingle),
                    Some("off") => Some(ButtonPress::BottomSingle),
                    _ => None,
                };

                if let Some(button_press) = button_press {
                    events.push(
                        UserTrigger::Remote(match target {
                            RemoteTarget::BedroomDoor => Remote::BedroomDoor(button_press),
                        })
                        .into(),
                    );
                }

                events
            }
        };

        Ok(result)
    }
}

fn availability(friendly_name: &str, last_seen: DateTime) -> IncomingData {
    ItemAvailability {
        source: "Z2M".to_string(),
        item: friendly_name.to_string(),
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
struct Thermostat {
    occupied_heating_setpoint: f64,
    pi_heating_demand: f64,
    window_open_external: bool,
    load_estimate: i64,
    local_temperature: f64,
    external_measured_room_sensor: f64,
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

#[derive(Debug, Clone, serde::Deserialize)]
struct WaterLeakSensor {
    water_leak: bool,
    last_seen: DateTime,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RemoteControl {
    action: Option<String>,
    last_seen: DateTime,
}

#[derive(Debug)]
pub struct ThermostatGroup {
    first_id: String,
    first_factor: f64,
    first_data: Option<DataPoint<i64>>,
    second_id: String,
    second_factor: f64,
    second_data: Option<DataPoint<i64>>,
    last_sent_at: DateTime,
    last_sent_value: i64,
}

impl ThermostatGroup {
    pub fn new(first_id: String, first_factor: f64, second_id: String, second_factor: f64) -> Self {
        Self {
            first_id,
            first_factor,
            first_data: None,
            second_id,
            second_factor,
            second_data: None,
            last_sent_at: t!(24 hours ago),
            last_sent_value: -8000,
        }
    }

    pub async fn send_if_needed(&mut self, sender: &Z2mCommandExecutor) -> anyhow::Result<bool> {
        let mean = match self.mean() {
            Some(mean) => mean,
            None => {
                return Ok(false);
            }
        };

        if (mean - self.last_sent_value).abs() < 5 || t!(15 minutes ago) < self.last_sent_at {
            return Ok(false);
        }

        tracing::debug!(
            "Sending load estimate mean {} to thermostats {} and {}",
            mean,
            self.first_id,
            self.second_id
        );

        sender.set_load_room_mean(&self.first_id, mean).await?;
        sender.set_load_room_mean(&self.second_id, mean).await?;
        self.last_sent_at = t!(now);
        self.last_sent_value = mean;

        Ok(true)
    }

    fn register_load(&mut self, device_id: &str, data: DataPoint<i64>) {
        let data = if data.value > -500 { Some(data) } else { None };

        if device_id == self.first_id {
            self.first_data = data;
        } else if device_id == self.second_id {
            self.second_data = data;
        }
    }

    fn mean(&self) -> Option<i64> {
        match (&self.first_data, &self.second_data) {
            (Some(first), Some(second)) => {
                //weighted by influence of the radiator
                let v = (self.first_factor * (first.value as f64) + self.second_factor * (second.value as f64))
                    / (self.first_factor + self.second_factor);
                Some(v.round() as i64)
            }
            _ => None,
        }
    }
}
