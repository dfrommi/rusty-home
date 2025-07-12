mod command;
mod mapper;
mod state;

use crate::home::state::{EnergySaving, FanActivity, FanAirflow, Powered};
use serde::{Deserialize, Serialize};

use crate::Infrastructure;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Homekit {
    pub base_topic_status: String,
    pub base_topic_set: String,
}

impl Homekit {
    pub fn export_state(&self, infrastructure: &Infrastructure) -> impl Future<Output = ()> + use<> {
        let mqtt_api = infrastructure.api.clone();
        let mqtt_sender = infrastructure.mqtt_client.new_publisher();
        let state_topic = self.base_topic_status.clone();
        let mqtt_trigger = infrastructure.event_listener.new_state_changed_listener();

        async move { state::export_state(&mqtt_api, state_topic, mqtt_sender, mqtt_trigger).await }
    }

    pub async fn process_commands(&self, infrastructure: &mut Infrastructure) -> impl Future<Output = ()> + use<> {
        let mqtt_command_receiver = infrastructure
            .mqtt_client
            .subscribe(format!("{}/#", &self.base_topic_set))
            .await
            .expect("Error subscribing to MQTT topic");
        let api = infrastructure.api.clone();
        let target_topic = self.base_topic_set.clone();

        async move { command::process_commands(target_topic, mqtt_command_receiver, api).await }
    }
}

#[derive(Debug, Clone)]
struct HomekitStateValue(String);

#[derive(Debug, Clone)]
enum HomekitState {
    Powered(Powered),
    EnergySaving(EnergySaving),
    FanSpeed(FanActivity),
}

//Don't forget to add to action planning config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum HomekitCommand {
    InfraredHeaterPower(bool),
    DehumidifierPower(bool),
    LivingRoomTvEnergySaving(bool),
    LivingRoomCeilingFanSpeed(FanAirflow),
    BedroomCeilingFanSpeed(FanAirflow),
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Homekit[{}]", _variant)]
pub enum HomekitCommandTarget {
    InfraredHeaterPower,
    DehumidifierPower,
    LivingRoomTvEnergySaving,
    LivingRoomCeilingFanSpeed,
    BedroomCeilingFanSpeed,
}

impl Homekit {
    fn config() -> Vec<(&'static str, HomekitState, Option<HomekitCommandTarget>)> {
        vec![
            (
                "powered/infared_heater",
                HomekitState::Powered(Powered::InfraredHeater),
                Some(HomekitCommandTarget::InfraredHeaterPower),
            ),
            (
                "powered/dehumidifier",
                HomekitState::Powered(Powered::Dehumidifier),
                Some(HomekitCommandTarget::DehumidifierPower),
            ),
            (
                "energy_saving/living_room_tv",
                HomekitState::EnergySaving(EnergySaving::LivingRoomTv),
                Some(HomekitCommandTarget::LivingRoomTvEnergySaving),
            ),
            (
                "fan_speed/bedroom_ceiling_fan",
                HomekitState::FanSpeed(FanActivity::BedroomCeilingFan),
                Some(HomekitCommandTarget::BedroomCeilingFanSpeed),
            ),
            (
                "fan_speed/living_room_ceiling_fan",
                HomekitState::FanSpeed(FanActivity::LivingRoomCeilingFan),
                Some(HomekitCommandTarget::LivingRoomCeilingFanSpeed),
            ),
        ]
    }
}
