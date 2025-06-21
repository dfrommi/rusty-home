use anyhow::bail;
use api::{
    state::{Powered, unit::FanAirflow},
    trigger::{Homekit, UserTrigger},
};
use infrastructure::MqttInMessage;
use support::{ExternalId, unit::Percent};
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
    let external_id = ExternalId::new(type_name, item_name);

    if let Ok(powered) = Powered::try_from(&external_id) {
        return match powered {
            Powered::Dehumidifier => Ok(UserTrigger::Homekit(Homekit::DehumidifierPower(
                value.try_into()?,
            ))),
            Powered::InfraredHeater => Ok(UserTrigger::Homekit(Homekit::InfraredHeaterPower(
                value.try_into()?,
            ))),
            _ => bail!("Powered-item {} not supported", item_name),
        };
    }

    if let Ok(energy_saving) = EnergySaving::try_from(&external_id) {
        return match energy_saving {
            EnergySaving::LivingRoomTv => Ok(UserTrigger::Homekit(
                Homekit::LivingRoomTvEnergySaving(value.try_into()?),
            )),
        };
    }

    if type_name == "fan_speed" {
        let percent: Percent = value.clone().try_into()?;
        let activity = if percent.0 == 0.0 {
            FanAirflow::Off
        } else {
            FanAirflow::Forward(value.try_into()?)
        };

        if item_name == "living_room_ceiling_fan" {
            return Ok(UserTrigger::Homekit(Homekit::LivingRoomCeilingFanSpeed(
                activity,
            )));
        }
    }

    bail!("Device {}/{} not supported", type_name, item_name)
}
