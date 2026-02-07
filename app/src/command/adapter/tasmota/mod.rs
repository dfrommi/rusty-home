mod config;

use super::CommandExecutor;

use crate::command::{Command, CommandTarget};

use super::metrics::*;
use infrastructure::MqttSender;

#[derive(Debug, Clone)]
enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}

pub struct TasmotaCommandExecutor {
    config: Vec<(CommandTarget, TasmotaCommandTarget)>,
    sender: MqttSender,
}

impl TasmotaCommandExecutor {
    pub fn new(mqtt_sender: MqttSender) -> TasmotaCommandExecutor {
        let config = config::default_tasmota_command_config();

        Self {
            config,
            sender: mqtt_sender,
        }
    }
}

impl CommandExecutor for TasmotaCommandExecutor {
    #[tracing::instrument(name = "execute_command TASMOTA", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let cmd_target: CommandTarget = command.into();
        let tasmota_target = self
            .config
            .iter()
            .find_map(|(cmd, tasmota)| if cmd == &cmd_target { Some(tasmota) } else { None });

        if tasmota_target.is_none() {
            return Ok(false);
        }

        match (command, tasmota_target.unwrap()) {
            (Command::SetPower { power_on, .. }, TasmotaCommandTarget::PowerSwitch(device_id)) => {
                self.sender
                    .send_transient(
                        format!("cmnd/{}/Power1", device_id),
                        if *power_on { "ON".to_string() } else { "OFF".to_string() },
                    )
                    .await?;

                CommandMetric::Executed {
                    device_id: device_id.to_string(),
                    system: CommandTargetSystem::Tasmota,
                }
                .record();

                Ok(true)
            }

            (_, tasmota_target) => {
                anyhow::bail!("Mismatch between command and tasmota target {:?}", tasmota_target)
            }
        }
    }
}
