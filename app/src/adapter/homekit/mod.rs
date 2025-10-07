mod command;
mod mapper;
mod state;

use crate::{
    core::unit::DegreeCelsius,
    home::{
        HeatingZone,
        state::{
            EnergySaving, FanActivity, FanAirflow, OpenedArea, Powered, RelativeHumidity, SetPoint, Temperature,
            UserControlled,
        },
    },
};
use r#macro::{EnumVariants, Id};
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
    CurrentTemperature(Temperature),
    CurrentHumidity(RelativeHumidity),
    CurrentHeatingState(SetPoint, UserControlled),
    TargetTemperature(SetPoint),
    WindowOpen(OpenedArea),
}

#[derive(Debug, Clone, derive_more::Display)]
pub enum HomekitInput {
    InfraredHeaterPower,
    DehumidifierPower,
    LivingRoomTvEnergySaving,
    LivingRoomCeilingFanSpeed,
    BedroomCeilingFanSpeed,
    ThermostatTargetHeatingState(HeatingZone),
    ThermostatTargetTemperature(HeatingZone),
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
    LivingRoomHeatingState(HomekitHeatingState),
    BedroomHeatingState(HomekitHeatingState),
    KitchenHeatingState(HomekitHeatingState),
    RoomOfRequirementsHeatingState(HomekitHeatingState),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Homekit[{}]", _variant)]
pub enum HomekitCommandTarget {
    InfraredHeaterPower,
    DehumidifierPower,
    LivingRoomTvEnergySaving,
    LivingRoomCeilingFanSpeed,
    BedroomCeilingFanSpeed,
    LivingRoomHeatingState,
    BedroomHeatingState,
    KitchenHeatingState,
    RoomOfRequirementsHeatingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HomekitHeatingState {
    Off,
    Heat(DegreeCelsius),
    Auto,
}

impl Homekit {
    fn config() -> Vec<(&'static str, HomekitState, Option<HomekitInput>)> {
        vec![
            (
                "powered/infared_heater",
                HomekitState::Powered(Powered::InfraredHeater),
                Some(HomekitInput::InfraredHeaterPower),
            ),
            (
                "powered/dehumidifier",
                HomekitState::Powered(Powered::Dehumidifier),
                Some(HomekitInput::DehumidifierPower),
            ),
            (
                "energy_saving/living_room_tv",
                HomekitState::EnergySaving(EnergySaving::LivingRoomTv),
                Some(HomekitInput::LivingRoomTvEnergySaving),
            ),
            (
                "fan_speed/bedroom_ceiling_fan",
                HomekitState::FanSpeed(FanActivity::BedroomCeilingFan),
                Some(HomekitInput::BedroomCeilingFanSpeed),
            ),
            (
                "fan_speed/living_room_ceiling_fan",
                HomekitState::FanSpeed(FanActivity::LivingRoomCeilingFan),
                Some(HomekitInput::LivingRoomCeilingFanSpeed),
            ),
            (
                "contact/living_room/closed",
                HomekitState::WindowOpen(OpenedArea::LivingRoomWindowOrDoor),
                None,
            ),
            (
                "contact/bedroom/closed",
                HomekitState::WindowOpen(OpenedArea::BedroomWindow),
                None,
            ),
            (
                "contact/room_of_requirements/closed",
                HomekitState::WindowOpen(OpenedArea::RoomOfRequirementsWindow),
                None,
            ),
            (
                "contact/kitchen/closed",
                HomekitState::WindowOpen(OpenedArea::KitchenWindow),
                None,
            ),
            (
                "thermostat/living_room/current_temperature",
                HomekitState::CurrentTemperature(Temperature::LivingRoom),
                None,
            ),
            (
                "thermostat/bedroom/current_temperature",
                HomekitState::CurrentTemperature(Temperature::Bedroom),
                None,
            ),
            (
                "thermostat/room_of_requirements/current_temperature",
                HomekitState::CurrentTemperature(Temperature::RoomOfRequirements),
                None,
            ),
            (
                "thermostat/kitchen/current_temperature",
                HomekitState::CurrentTemperature(Temperature::Kitchen),
                None,
            ),
            (
                "thermostat/living_room/current_humidity",
                HomekitState::CurrentHumidity(RelativeHumidity::LivingRoom),
                None,
            ),
            (
                "thermostat/bedroom/current_humidity",
                HomekitState::CurrentHumidity(RelativeHumidity::Bedroom),
                None,
            ),
            (
                "thermostat/room_of_requirements/current_humidity",
                HomekitState::CurrentHumidity(RelativeHumidity::RoomOfRequirements),
                None,
            ),
            (
                "thermostat/kitchen/current_humidity",
                HomekitState::CurrentHumidity(RelativeHumidity::Kitchen),
                None,
            ),
            (
                "thermostat/living_room/heating_state",
                HomekitState::CurrentHeatingState(SetPoint::LivingRoomBig, UserControlled::LivingRoomThermostatBig),
                Some(HomekitInput::ThermostatTargetHeatingState(HeatingZone::LivingRoom)),
            ),
            (
                "thermostat/bedroom/heating_state",
                HomekitState::CurrentHeatingState(SetPoint::Bedroom, UserControlled::BedroomThermostat),
                Some(HomekitInput::ThermostatTargetHeatingState(HeatingZone::Bedroom)),
            ),
            (
                "thermostat/room_of_requirements/heating_state",
                HomekitState::CurrentHeatingState(
                    SetPoint::RoomOfRequirements,
                    UserControlled::RoomOfRequirementsThermostat,
                ),
                Some(HomekitInput::ThermostatTargetHeatingState(HeatingZone::RoomOfRequirements)),
            ),
            (
                "thermostat/kitchen/heating_state",
                HomekitState::CurrentHeatingState(SetPoint::Kitchen, UserControlled::KitchenThermostat),
                Some(HomekitInput::ThermostatTargetHeatingState(HeatingZone::Kitchen)),
            ),
            (
                "thermostat/living_room/target_temperature",
                HomekitState::TargetTemperature(SetPoint::LivingRoomBig),
                Some(HomekitInput::ThermostatTargetTemperature(HeatingZone::LivingRoom)),
            ),
            (
                "thermostat/bedroom/target_temperature",
                HomekitState::TargetTemperature(SetPoint::Bedroom),
                Some(HomekitInput::ThermostatTargetTemperature(HeatingZone::Bedroom)),
            ),
            (
                "thermostat/room_of_requirements/target_temperature",
                HomekitState::TargetTemperature(SetPoint::RoomOfRequirements),
                Some(HomekitInput::ThermostatTargetTemperature(HeatingZone::RoomOfRequirements)),
            ),
            (
                "thermostat/kitchen/target_temperature",
                HomekitState::TargetTemperature(SetPoint::Kitchen),
                Some(HomekitInput::ThermostatTargetTemperature(HeatingZone::Kitchen)),
            ),
        ]
    }
}
