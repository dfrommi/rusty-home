use infrastructure::MqttSender;

use super::metrics::{CommandMetric, CommandTargetSystem};

pub struct LgTvCommandExecutor {
    sender: MqttSender,
}

impl LgTvCommandExecutor {
    pub fn new(sender: MqttSender) -> Self {
        Self { sender }
    }

    pub async fn set_energy_saving(&self, on: bool) -> anyhow::Result<()> {
        self.sender
            .send_transient("command/energySaving", energy_saving_payload(on))
            .await?;

        CommandMetric::Executed {
            device_id: "lgtv".to_string(),
            system: CommandTargetSystem::LGTV,
        }
        .record();

        Ok(())
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
