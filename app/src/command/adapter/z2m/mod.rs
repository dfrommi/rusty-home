mod config;

use crate::{
    command::{Command, CommandTarget, adapter::CommandExecutor},
    core::unit::{DegreeCelsius, Percent},
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
            (Command::SetThermostatValveOpeningPosition { value, .. }, Z2mCommandTarget::Thermostat(device_id)) => {
                self.set_valve_opening_position(device_id, *value).await
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

#[derive(Debug, serde::Serialize)]
struct ThermostatLoadPayload {
    load_room_mean: i64,
}
