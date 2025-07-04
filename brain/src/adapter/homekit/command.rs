use std::collections::HashMap;

use crate::home::state::{FanActivity, FanAirflow, Powered};
use crate::home::trigger::{Homekit, UserTrigger};
use anyhow::bail;
use infrastructure::MqttInMessage;
use support::{ExternalId, unit::Percent};
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use crate::home::state::EnergySaving;

use super::MqttStateValue;

pub async fn process_commands(
    base_topic: String,
    mut rx: Receiver<MqttInMessage>,
    api: crate::Database,
) {
    let mut debounce_tasks: HashMap<String, JoinHandle<()>> = HashMap::new();

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

        if let Some(handle) = debounce_tasks.remove(&msg.topic) {
            tracing::debug!(
                "Received command for already scheduled command on topic {}, aborting previous task",
                msg.topic,
            );
            handle.abort();
        }

        let type_name = topic_parts[1].to_string();
        let item_name = topic_parts[2].to_string();

        let schedule_api = api.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            match to_command(&type_name, &item_name, MqttStateValue(msg.payload)) {
                Ok(trigger) => {
                    tracing::info!("Executing command received via Mqtt: {:?}", trigger);
                    if let Err(e) = schedule_api.add_user_trigger(trigger).await {
                        tracing::error!("Error triggering user action: {:?}", e)
                    }
                }
                Err(e) => tracing::error!("{}", e),
            }
        });

        debounce_tasks.insert(msg.topic.clone(), handle);
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

        if item_name == FanActivity::LivingRoomCeilingFan.ext_name() {
            return Ok(UserTrigger::Homekit(Homekit::LivingRoomCeilingFanSpeed(
                activity,
            )));
        }

        if item_name == FanActivity::BedroomCeilingFan.ext_name() {
            return Ok(UserTrigger::Homekit(Homekit::BedroomCeilingFanSpeed(
                activity,
            )));
        }
    }

    bail!("Device {}/{} not supported", type_name, item_name)
}
