use super::metrics::{CommandMetric, CommandTargetSystem};
use infrastructure::MqttSender;

pub struct TasmotaCommandExecutor {
    sender: MqttSender,
}

impl TasmotaCommandExecutor {
    pub fn new(mqtt_sender: MqttSender) -> Self {
        Self { sender: mqtt_sender }
    }

    pub async fn set_power(&self, device_id: &str, power_on: bool) -> anyhow::Result<()> {
        self.sender
            .send_transient(
                format!("cmnd/{}/Power1", device_id),
                if power_on { "ON".to_string() } else { "OFF".to_string() },
            )
            .await?;

        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Tasmota,
        }
        .record();

        Ok(())
    }
}
