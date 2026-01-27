mod config;

use super::metrics::*;
use crate::{
    command::{Command, CommandTarget, adapter::CommandExecutor},
    core::unit::Percent,
};
use infrastructure::MqttSender;
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    SonoffThermostat(&'static str),
    PowerPlug(&'static str),
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
                Command::SetThermostatValveOpeningPosition { value, .. },
                Z2mCommandTarget::SonoffThermostat(device_id),
            ) => self.set_valve_opening_position_sonoff(device_id, *value).await,
            (Command::SetPower { power_on, .. }, Z2mCommandTarget::PowerPlug(device_id)) => {
                self.set_power_state(device_id, *power_on).await
            }
            (_, z2m_target) => {
                anyhow::bail!("Mismatch between command and Z2M target {:?}", z2m_target)
            }
        }
    }
}

impl Z2mCommandExecutor {
    pub async fn set_valve_opening_position_sonoff(&self, device_id: &str, value: Percent) -> anyhow::Result<bool> {
        let (system_mode, setpoint) = if value.0 > 0.0 { ("heat", 35.0) } else { ("off", 4.0) };
        let opened_percentage = (value.0.round() as i64).clamp(0, 100);
        //use always full max closing instead of `100 - opened_percentage` as it might avoid loosing
        //calibration over time and shouldn't have any impact on when it's actually closed
        let closed_percentage = 100;

        self.send_message(
            device_id,
            json!({
                "system_mode": system_mode,
                "valve_opening_degree": opened_percentage,
                "valve_closing_degree": closed_percentage,
                "occupied_heating_setpoint": setpoint,
            }),
        )
        .await?;

        Ok(true)
    }

    pub async fn set_power_state(&self, device_id: &str, power_on: bool) -> anyhow::Result<bool> {
        let power_state = if power_on { "ON" } else { "OFF" };

        self.send_message(
            device_id,
            json!({
                "state": power_state,
            }),
        )
        .await?;

        Ok(true)
    }

    async fn send_message(&self, device_id: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        self.sender
            .send_transient(self.target_topic(device_id), payload.to_string())
            .await?;

        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Z2M,
        }
        .record();

        Ok(())
    }
}
