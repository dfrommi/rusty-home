use api::{
    state::Powered,
    trigger::{Homekit, UserTrigger},
};
use support::mqtt::MqttInMessage;
use tokio::sync::mpsc::Receiver;

use crate::{home::state::EnergySaving, port::UserTriggerExecutor};

use super::MqttStateValue;

pub async fn process_commands(
    base_topic: String,
    mut rx: Receiver<MqttInMessage>,
    api: &impl UserTriggerExecutor,
) {
    while let Some(msg) = rx.recv().await {
        let topic_parts: Vec<&str> = msg
            .topic
            .strip_prefix(&base_topic)
            .unwrap_or("")
            .split('/')
            .collect();

        if topic_parts.len() != 3 {
            tracing::warn!("Received malformed topic: {}", msg.topic);
            continue;
        }

        let type_name = topic_parts[1];
        let item_name = topic_parts[2];

        match to_command(type_name, item_name, MqttStateValue(msg.payload)) {
            Ok(trigger) => {
                tracing::info!("Executing command received via Mqtt: {:?}", trigger);
                if let Err(e) = api.add_user_trigger(trigger).await {
                    tracing::error!("Error triggering user action: {:?}", e)
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
}

fn to_command(
    type_name: &str,
    item_name: &str,
    value: MqttStateValue,
) -> anyhow::Result<UserTrigger> {
    match type_name {
        Powered::TYPE_NAME => match Powered::from_item_name(item_name) {
            Some(Powered::Dehumidifier) => Ok(UserTrigger::Homekit(Homekit::DehumidifierPower(
                value.try_into()?,
            ))),
            Some(Powered::InfraredHeater) => Ok(UserTrigger::Homekit(
                Homekit::InfraredHeaterPower(value.try_into()?),
            )),
            Some(_) => Err(anyhow::anyhow!("Powered-item {} not supported", item_name)),
            None => Err(anyhow::anyhow!("Powered-item {} not found", item_name)),
        },
        EnergySaving::TYPE_NAME => match EnergySaving::from_item_name(item_name) {
            Some(EnergySaving::LivingRoomTv) => Ok(UserTrigger::Homekit(
                Homekit::LivingRoomTvEnergySaving(value.try_into()?),
            )),
            None => Err(anyhow::anyhow!("EnergySaving-item {} not found", item_name)),
        },
        _ => Err(anyhow::anyhow!(
            "Device {} channel {} not supported",
            type_name,
            item_name
        )),
    }
}
