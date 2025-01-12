use api::command::{Command, CommandTarget};
use support::mqtt::MqttOutMessage;

use crate::core::CommandExecutor;

use super::TasmotaCommandTarget;

pub struct TasmotaCommandExecutor {
    base_topic: String,
    config: Vec<(CommandTarget, TasmotaCommandTarget)>,
    sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
}

impl TasmotaCommandExecutor {
    pub fn new(
        base_topic: String,
        config: Vec<(CommandTarget, TasmotaCommandTarget)>,
        sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
    ) -> Self {
        Self {
            base_topic,
            config,
            sender,
        }
    }
}

impl CommandExecutor for TasmotaCommandExecutor {
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let cmd_target: CommandTarget = command.into();
        let tasmota_target = self.config.iter().find_map(|(cmd, tasmota)| {
            if cmd == &cmd_target {
                Some(tasmota)
            } else {
                None
            }
        });

        if tasmota_target.is_none() {
            return Ok(false);
        }

        match (command, tasmota_target.unwrap()) {
            (Command::SetPower { power_on, .. }, TasmotaCommandTarget::PowerSwitch(device_id)) => {
                let msg = MqttOutMessage {
                    topic: format!("{}/cmnd/{}/Power1", self.base_topic, device_id),
                    payload: if *power_on {
                        "ON".to_string()
                    } else {
                        "OFF".to_string()
                    },
                    retain: false,
                };

                self.sender.send(msg).await?;
                Ok(true)
            }

            (_, tasmota_target) => {
                anyhow::bail!(
                    "Mismatch between command and tasmota target {:?}",
                    tasmota_target
                )
            }
        }
    }
}
