mod command;
mod config;
mod event;

pub use command::to_command_payload;
use event::{on_ha_event_received, persist_current_ha_state};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::settings;
use anyhow::Result;
use api::{
    command::CommandExecution,
    state::{
        CurrentPowerUsage, Opened, Powered, RelativeHumidity, Temperature, TotalEnergyConsumption,
    },
};
use support::mqtt::{MqttInMessage, MqttOutMessage};

use super::persistence::BackendApi;

#[derive(Debug, Clone)]
enum HaChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Opened(Opened),
    Powered(Powered),
    CurrentPowerUsage(CurrentPowerUsage),
    TotalEnergyConsumption(TotalEnergyConsumption),
}

enum HomeAssistantService {
    SwitchTurnOn,
    SwitchTurnOff,
}

struct HaCommandEntity<'a> {
    pub id: &'a str,
    pub service: HomeAssistantService,
}

pub async fn process_ha_events(
    api: &BackendApi,
    mut event_rx: Receiver<MqttInMessage>,
    settings: &settings::HomeAssitant,
) -> Result<()> {
    persist_current_ha_state(api, &settings.url, &settings.token).await?;

    tracing::info!("Start processing HA events");
    while let Some(msg) = event_rx.recv().await {
        on_ha_event_received(api, msg).await;
    }

    Ok(())
}

pub async fn process_ha_commands(
    mut cmd_rx: Receiver<CommandExecution>,
    mqtt_sender: Sender<MqttOutMessage>,
    command_mqtt_topic: &str,
    api: &BackendApi,
) {
    while let Some(command_execution) = cmd_rx.recv().await {
        tracing::info!("Processing command: {:?}", command_execution);

        match to_command_payload(&command_execution.command) {
            Some(payload) => {
                let mqtt_msg = MqttOutMessage {
                    topic: command_mqtt_topic.to_string(),
                    payload,
                    retain: false,
                };
                match mqtt_sender.send(mqtt_msg).await {
                    Ok(_) => {
                        if let Err(e) = api.set_command_state_success(command_execution.id).await {
                            tracing::error!("Error setting command state to SUCESS in DB: {}", e);
                        }
                    }
                    Err(e) => {
                        if let Err(e) = api
                            .set_command_state_error(
                                command_execution.id,
                                format!("Error sending command to MQTT: {}", e).as_str(),
                            )
                            .await
                        {
                            tracing::error!("Error setting command state to ERROR in DB: {}", e);
                        }
                    }
                }
            }
            None => {
                tracing::trace!(
                    "Command not supported by Home Assistant: {:?}",
                    command_execution
                );
            }
        }
    }
}
