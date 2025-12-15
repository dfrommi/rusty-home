use crate::{
    adapter::command::CommandExecutor,
    command::{Command, CommandTarget},
};
use infrastructure::MqttOutMessage;

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
                let msg = MqttOutMessage::transient(
                    format!("{}/cmnd/{}/Power1", self.base_topic, device_id),
                    if *power_on { "ON".to_string() } else { "OFF".to_string() },
                );

                self.sender.send(msg).await?;
                Ok(true)
            }

            (_, tasmota_target) => {
                anyhow::bail!("Mismatch between command and tasmota target {:?}", tasmota_target)
            }
        }
    }
}
