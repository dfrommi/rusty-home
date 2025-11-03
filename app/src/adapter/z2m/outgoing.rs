use crate::{
    adapter::command::CommandExecutor,
    core::unit::DegreeCelsius,
    home::command::{Command, CommandTarget, HeatingTargetState},
};
use infrastructure::MqttOutMessage;
use serde_json::json;

use super::Z2mCommandTarget;

pub struct Z2mCommandExecutor {
    base_topic: String,
    config: Vec<(CommandTarget, Z2mCommandTarget)>,
    sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
}

impl Z2mCommandExecutor {
    pub fn new(
        base_topic: String,
        config: Vec<(CommandTarget, Z2mCommandTarget)>,
        sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
    ) -> Self {
        Self {
            base_topic,
            config,
            sender,
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
            ) => self.set_heating(device_id, None, true).await,
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

        let msg = MqttOutMessage::transient(
            self.target_topic(device_id),
            serde_json::to_string(&ThermostatCommandPayload {
                window_open_external: window_open,
                occupied_heating_setpoint: high_priority_setpoint.map(|t| t.temperature.0),
                occupied_heating_setpoint_scheduled: low_priority_setpoint.map(|t| t.temperature.0),
            })?,
        );

        self.sender.send(msg).await?;
        Ok(true)
    }

    async fn set_ambient_temperature(&self, device_id: &str, temperature: DegreeCelsius) -> anyhow::Result<bool> {
        let value = (temperature.0 * 100.0) as i32; //Z2M expects temperature in centi-degrees

        let msg = MqttOutMessage::transient(
            self.target_topic(device_id),
            json!({ "external_measured_room_sensor": value }).to_string(),
        );

        self.sender.send(msg).await?;
        Ok(true)
    }

    pub async fn set_load_room_mean(&self, device_id: &str, load: i64) -> anyhow::Result<bool> {
        let msg = MqttOutMessage::transient(
            self.target_topic(device_id),
            serde_json::to_string(&ThermostatLoadPayload { load_room_mean: load })?,
        );

        self.sender.send(msg).await?;
        Ok(true)
    }
}

//TODO occupied_heating_setpoint_scheduled
#[derive(Debug, serde::Serialize)]
struct ThermostatCommandPayload {
    window_open_external: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupied_heating_setpoint: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupied_heating_setpoint_scheduled: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
struct ThermostatLoadPayload {
    load_room_mean: i64,
}
