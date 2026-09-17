use infrastructure::MqttSender;

use crate::command::{Command, adapter::CommandExecutor};

use super::metrics::{CommandMetric, CommandTargetSystem};

pub struct LgTvCommandExecutor {
    sender: MqttSender,
}

impl LgTvCommandExecutor {
    pub fn new(sender: MqttSender) -> Self {
        Self { sender }
    }
}

impl CommandExecutor for LgTvCommandExecutor {
    #[tracing::instrument(name = "execute_command LGTV", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let Command::SetEnergySaving { on, .. } = command else {
            return Ok(false);
        };

        self.sender
            .send_transient("command/energySaving", energy_saving_payload(*on))
            .await?;

        CommandMetric::Executed {
            device_id: "lgtv".to_string(),
            system: CommandTargetSystem::LGTV,
        }
        .record();

        Ok(true)
    }
}

fn energy_saving_payload(on: bool) -> String {
    if on { "auto" } else { "off" }.to_string()
}

#[cfg(test)]
mod tests {
    use super::energy_saving_payload;

    #[test]
    fn maps_energy_saving_state_to_lg_tv_command_payload() {
        assert_eq!(energy_saving_payload(true), "auto");
        assert_eq!(energy_saving_payload(false), "off");
    }
}
