use std::collections::HashMap;
use std::sync::Arc;

use super::{Homekit, HomekitCommand, HomekitCommandTarget};
use crate::core::HomeApi;
use crate::core::unit::Percent;
use crate::home::state::FanAirflow;
use crate::home::trigger::UserTrigger;
use infrastructure::MqttInMessage;
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use super::HomekitStateValue;

pub async fn process_commands(base_topic: String, mut rx: Receiver<MqttInMessage>, api: HomeApi) {
    let mut debounce_tasks: HashMap<String, JoinHandle<()>> = HashMap::new();
    let api = Arc::new(api);

    while let Some(msg) = rx.recv().await {
        let topic = msg.topic.clone();

        if let Some(handle) = debounce_tasks.remove(&topic) {
            tracing::trace!(
                "Received command for already scheduled command on topic {}, aborting previous task",
                topic,
            );
            handle.abort();
        }

        let schedule_api = api.clone();
        let scheedule_base_topic = base_topic.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            handle_message(&scheedule_base_topic, msg, schedule_api).await;
        });

        debounce_tasks.insert(topic, handle);
    }
}

async fn handle_message(base_topic: &str, msg: MqttInMessage, api: Arc<HomeApi>) {
    let config = Homekit::config();
    let target: Option<&HomekitCommandTarget> = config.iter().find_map(|(key, _, target)| {
        if msg.topic == format!("{}/{}", base_topic, key) {
            return target.as_ref();
        } else {
            return None;
        };
    });

    if let Some(target) = target {
        tracing::info!("Received command for {}", target);
        if let Err(e) = execute_target(target, HomekitStateValue(msg.payload), api).await {
            tracing::error!("Error triggering command for {}: {:?}", target, e);
        }
    } else {
        tracing::warn!("No command target configured for topic {}", msg.topic);
    }
}

async fn execute_target(
    target: &HomekitCommandTarget,
    payload: HomekitStateValue,
    api: Arc<HomeApi>,
) -> anyhow::Result<()> {
    match target {
        HomekitCommandTarget::InfraredHeaterPower => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(payload.try_into()?)))
                .await
        }
        HomekitCommandTarget::DehumidifierPower => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::DehumidifierPower(payload.try_into()?)))
                .await
        }
        HomekitCommandTarget::LivingRoomTvEnergySaving => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(
                payload.try_into()?,
            )))
            .await
        }
        HomekitCommandTarget::LivingRoomCeilingFanSpeed | HomekitCommandTarget::BedroomCeilingFanSpeed => {
            let percent: Percent = payload.clone().try_into()?;
            let activity = if percent.0 == 0.0 {
                FanAirflow::Off
            } else {
                FanAirflow::Forward(payload.try_into()?)
            };
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::LivingRoomCeilingFanSpeed(activity)))
                .await
        }
    }
}
