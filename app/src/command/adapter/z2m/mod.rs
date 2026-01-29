mod config;
pub mod sender;

use super::metrics::*;
use crate::{
    command::{Command, CommandTarget, adapter::CommandExecutor},
    core::unit::Percent,
};
use sender::Z2mSender;
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    SonoffThermostat(&'static str),
    PowerPlug(&'static str),
}

pub struct Z2mCommandExecutor {
    config: Vec<(CommandTarget, Z2mCommandTarget)>,
    sender: Z2mSender,
}

impl Z2mCommandExecutor {
    pub fn new(mqtt_sender: Z2mSender) -> Self {
        let config = config::default_z2m_command_config();
        Self {
            config,
            sender: mqtt_sender,
        }
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
        let opened_percentage = (value.0.round() as i64).clamp(0, 100);

        let payloads = if opened_percentage > 0 {
            vec![
                json!({
                    "valve_opening_degree": opened_percentage,
                }),
                json!({
                    "system_mode": "heat",
                    "occupied_heating_setpoint": 35,
                }),
            ]
        } else {
            vec![
                json!({
                    "valve_opening_degree": opened_percentage,
                }),
                // Goes automatically to frost protection temperature
                json!({
                    "system_mode": "off",
                }),
            ]
        };

        self.send_message(device_id, payloads, false).await?;

        Ok(true)
    }

    pub async fn set_power_state(&self, device_id: &str, power_on: bool) -> anyhow::Result<bool> {
        let power_state = if power_on { "ON" } else { "OFF" };

        self.send_message(
            device_id,
            vec![json!({
                "state": power_state,
            })],
            true,
        )
        .await?;

        Ok(true)
    }

    async fn send_message(
        &self,
        device_id: &str,
        payloads: Vec<serde_json::Value>,
        optimistic: bool,
    ) -> anyhow::Result<()> {
        self.sender.send(device_id, payloads, optimistic).await?;

        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Z2M,
        }
        .record();

        Ok(())
    }
}
