use api::command::{Command, PowerToggle};
use support::mqtt::MqttInMessage;
use tokio::sync::mpsc::Receiver;

use crate::thing::Executable;

pub async fn process_commands(base_topic: &str, mut rx: Receiver<MqttInMessage>) {
    while let Some(msg) = rx.recv().await {
        let topic_parts: Vec<&str> = msg
            .topic
            .strip_prefix(base_topic)
            .unwrap_or("")
            .split('/')
            .collect();

        if topic_parts.len() != 3 {
            tracing::warn!("Received malformed topic: {}", msg.topic);
            continue;
        }

        let name = topic_parts[1];
        let channel = topic_parts[2];

        match to_command(name, channel, &msg.payload) {
            Ok(command) => {
                tracing::info!("Executing command received via Mqtt: {:?}", command);
                if let Err(e) = command.execute().await {
                    tracing::error!("Error executing command: {:?}", e)
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
}

fn to_command(name: &str, channel: &str, payload: &str) -> Result<Command, String> {
    match (name, channel) {
        ("dehumidifier", "power") => Ok(Command::SetPower {
            item: PowerToggle::Dehumidifier,
            power_on: try_bool(payload)?,
        }),
        _ => Err(format!("Device {} channel {} not supported", name, channel)),
    }
}

fn try_bool(payload: &str) -> Result<bool, String> {
    match payload {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(format!("Error converting {} to bool", payload)),
    }
}
