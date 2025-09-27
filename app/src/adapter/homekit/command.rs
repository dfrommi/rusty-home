use std::collections::HashMap;
use std::sync::Arc;

use super::{Homekit, HomekitCommand};
use crate::adapter::homekit::HomekitHeatingState;
use crate::core::unit::Percent;
use crate::home::command::Thermostat;
use crate::home::state::FanAirflow;
use crate::home::trigger::UserTrigger;
use crate::{adapter::homekit::HomekitInput, core::HomeApi};
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
    let input: Option<&HomekitInput> = config.iter().find_map(|(key, _, target)| {
        if msg.topic == format!("{base_topic}/{key}") {
            target.as_ref()
        } else {
            None
        }
    });

    if let Some(target) = input {
        tracing::info!("Received command for {}", target);
        if let Err(e) = execute_target(target, HomekitStateValue(msg.payload), api).await {
            tracing::error!("Error triggering command for {}: {:?}", target, e);
        }
    } else {
        tracing::warn!("No command target configured for topic {}", msg.topic);
    }
}

async fn execute_target(input: &HomekitInput, payload: HomekitStateValue, api: Arc<HomeApi>) -> anyhow::Result<()> {
    match input {
        HomekitInput::InfraredHeaterPower => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(payload.try_into()?)))
                .await
        }
        HomekitInput::DehumidifierPower => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::DehumidifierPower(payload.try_into()?)))
                .await
        }
        HomekitInput::LivingRoomTvEnergySaving => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::LivingRoomTvEnergySaving(
                payload.try_into()?,
            )))
            .await
        }
        HomekitInput::LivingRoomCeilingFanSpeed => {
            let activity: FanAirflow = payload.try_into()?;
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::LivingRoomCeilingFanSpeed(activity)))
                .await
        }
        HomekitInput::BedroomCeilingFanSpeed => {
            let activity: FanAirflow = payload.try_into()?;
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::BedroomCeilingFanSpeed(activity)))
                .await
        }
        HomekitInput::ThermostatTargetHeatingState(Thermostat::RoomOfRequirements) => {
            let heating_state = match payload.0.as_ref() {
                "OFF" => HomekitHeatingState::Off,
                "AUTO" => HomekitHeatingState::Auto,
                _ => return Ok(()), //HEAT is handled via target temperature
            };

            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(
                heating_state,
            )))
            .await
        }
        HomekitInput::ThermostatTargetTemperature(Thermostat::RoomOfRequirements) => {
            api.add_user_trigger(UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(
                HomekitHeatingState::Heat(payload.try_into()?),
            )))
            .await
        }
        HomekitInput::ThermostatTargetHeatingState(t) => {
            anyhow::bail!("Thermostat {t} heating state not yet supported by HomeKit")
        }
        HomekitInput::ThermostatTargetTemperature(t) => {
            anyhow::bail!("Thermostat {t} target temperature not yet supported by HomeKit")
        }
    }
}

impl TryInto<FanAirflow> for HomekitStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<FanAirflow, Self::Error> {
        let percent: Percent = self.clone().try_into()?;
        let activity = if percent.0 == 0.0 {
            FanAirflow::Off
        } else {
            FanAirflow::Forward(self.try_into()?)
        };
        Ok(activity)
    }
}
