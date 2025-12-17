mod config;

use crate::{
    command::{Command, CommandTarget, HeatingTargetState, adapter::CommandExecutor},
    core::unit::{DegreeCelsius, Percent, RawValue},
};
use infrastructure::MqttSender;
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    Thermostat(&'static str),
}

pub struct Z2mCommandExecutor {
    base_topic: String,
    config: Vec<(CommandTarget, Z2mCommandTarget)>,
    sender: MqttSender,
}

impl Z2mCommandExecutor {
    pub fn new(mqtt_sender: MqttSender, event_topic: &str) -> Self {
        let config = config::default_z2m_command_config();
        Self {
            base_topic: event_topic.to_string(),
            config,
            sender: mqtt_sender,
        }
    }

    fn target_topic(&self, device_id: &str) -> String {
        format!("{}/{}/set", self.base_topic, device_id)
    }
}

impl CommandExecutor for Z2mCommandExecutor {
    #[tracing::instrument(name = "execute_command Z2M", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let cmd_target: CommandTarget = command.into();
        let z2m_target = self
            .config
            .iter()
            .find_map(|(cmd, z2m)| if cmd == &cmd_target { Some(z2m) } else { None });

        if z2m_target.is_none() {
            return Ok(false);
        }

        match (command, z2m_target.unwrap()) {
            (
                Command::SetHeating {
                    target_state: HeatingTargetState::WindowOpen,
                    ..
                },
                Z2mCommandTarget::Thermostat(device_id),
            ) => {
                self.set_heating(
                    device_id,
                    Some(SetPoint {
                        temperature: DegreeCelsius(10.0),
                        low_priority: false,
                    }),
                    true,
                )
                .await
            }
            (
                Command::SetHeating {
                    target_state:
                        HeatingTargetState::Heat {
                            temperature,
                            low_priority,
                        },
                    ..
                },
                Z2mCommandTarget::Thermostat(device_id),
            ) => {
                self.set_heating(
                    device_id,
                    Some(SetPoint {
                        temperature: *temperature,
                        low_priority: *low_priority,
                    }),
                    false,
                )
                .await
            }
            (Command::SetThermostatAmbientTemperature { temperature, .. }, Z2mCommandTarget::Thermostat(device_id)) => {
                self.set_ambient_temperature(device_id, *temperature).await
            }
            (Command::SetThermostatValveOpeningPosition { value, .. }, Z2mCommandTarget::Thermostat(device_id)) => {
                self.set_valve_opening_position(device_id, *value).await
            }

            (Command::SetThermostatLoadMean { value, .. }, Z2mCommandTarget::Thermostat(device_id)) => {
                self.set_load_room_mean(device_id, *value).await
            }
            (_, z2m_target) => {
                anyhow::bail!("Mismatch between command and Z2M target {:?}", z2m_target)
            }
        }
    }
}

struct SetPoint {
    temperature: DegreeCelsius,
    low_priority: bool,
}

impl Z2mCommandExecutor {
    async fn set_heating(
        &self,
        device_id: &str,
        setpoint: Option<SetPoint>,
        window_open: bool,
    ) -> anyhow::Result<bool> {
        let (high_priority_setpoint, low_priority_setpoint) = match setpoint {
            Some(sp) if sp.low_priority => (None, Some(sp)),
            Some(sp) => (Some(sp), None),
            None => (None, None),
        };

        let operation_mode = if high_priority_setpoint.is_some() || low_priority_setpoint.is_some() {
            Some("setpoint".to_string())
        } else {
            None
        };

        self.sender
            .send_transient(
                self.target_topic(device_id),
                serde_json::to_string(&ThermostatCommandPayload {
                    window_open_external: window_open,
                    programming_operation_mode: operation_mode,
                    occupied_heating_setpoint: high_priority_setpoint.map(|t| t.temperature.0),
                    occupied_heating_setpoint_scheduled: low_priority_setpoint.map(|t| t.temperature.0),
                })?,
            )
            .await?;

        Ok(true)
    }

    async fn set_ambient_temperature(&self, device_id: &str, temperature: DegreeCelsius) -> anyhow::Result<bool> {
        let value = (temperature.0 * 100.0) as i32; //Z2M expects temperature in centi-degrees

        self.sender
            .send_transient(
                self.target_topic(device_id),
                json!({ "external_measured_room_sensor": value }).to_string(),
            )
            .await?;

        Ok(true)
    }

    pub async fn set_load_room_mean(&self, device_id: &str, value: RawValue) -> anyhow::Result<bool> {
        let value = value.0 as i64;

        self.sender
            .send_transient(
                self.target_topic(device_id),
                serde_json::to_string(&ThermostatLoadPayload { load_room_mean: value })?,
            )
            .await?;

        Ok(true)
    }

    pub async fn set_valve_opening_position(&self, device_id: &str, value: Percent) -> anyhow::Result<bool> {
        let system_mode = if value.0 > 0.0 { "heat" } else { "off" };
        let opened_percentage = (value.0.round() as i64).clamp(0, 100);
        let closed_percentage = 100 - opened_percentage;

        self.sender
            .send_transient(
                self.target_topic(device_id),
                json!({
                    "system_mode": system_mode,
                    "valve_opening_degree": opened_percentage,
                    "valve_closing_degree": closed_percentage,
                })
                .to_string(),
            )
            .await?;

        Ok(true)
    }
}

//TODO occupied_heating_setpoint_scheduled
#[derive(Debug, serde::Serialize)]
struct ThermostatCommandPayload {
    window_open_external: bool,
    programming_operation_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupied_heating_setpoint: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupied_heating_setpoint_scheduled: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
struct ThermostatLoadPayload {
    load_room_mean: i64,
}
