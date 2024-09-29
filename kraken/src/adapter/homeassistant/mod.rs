mod command;
mod config;
mod event;

pub use command::to_command_payload;
use event::{on_ha_event_received, persist_current_ha_state};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{error::Result, settings};
use api::{
    command::Command,
    state::{
        CurrentPowerUsage, Opened, Powered, RelativeHumidity, Temperature, TotalEnergyConsumption,
    },
    BackendApi,
};
use support::mqtt::{MqttInMessage, MqttOutMessage};

struct HaSensorEntity<'a> {
    pub id: &'a str,
    pub channel: HaChannel,
}

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
    persist_current_ha_state(api, &settings.url, &settings.token).await;

    let rx_api = api.clone();
    tokio::spawn(async move {
        while let Some(msg) = event_rx.recv().await {
            on_ha_event_received(&rx_api, msg).await;
        }
    });

    Ok(())
}

pub async fn process_ha_commands(
    mut cmd_rx: Receiver<Command>,
    mqtt_sender: Sender<MqttOutMessage>,
    command_mqtt_topic: &str,
) {
    while let Some(command) = cmd_rx.recv().await {
        match to_command_payload(&command) {
            Some(payload) => {
                let mqtt_msg = MqttOutMessage {
                    topic: command_mqtt_topic.to_string(),
                    payload,
                    retain: false,
                };
                mqtt_sender.send(mqtt_msg).await;
            }
            None => {
                tracing::trace!("Command not supported by Home Assistant: {:?}", command);
            }
        }
    }
}
