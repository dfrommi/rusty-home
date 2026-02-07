mod accessory;
mod hap;
mod runtime;

use infrastructure::EventListener;
use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use self::{
    accessory::HomekitRegistry,
    hap::{HomekitCharacteristic, HomekitService},
    runtime::HomekitRunner,
};
use crate::{
    Infrastructure,
    core::unit::{DegreeCelsius, FanAirflow, Percent},
    home_state::HomeStateEvent,
    trigger::TriggerClient,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HomekitTarget {
    pub(crate) name: String,
    pub(crate) service: HomekitService,
    pub(crate) characteristic: HomekitCharacteristic,
}

impl HomekitTarget {
    fn new(name: String, service: HomekitService, characteristic: HomekitCharacteristic) -> Self {
        Self {
            name,
            service,
            characteristic,
        }
    }

    pub(crate) fn into_config(self) -> HomekitTargetConfig {
        HomekitTargetConfig {
            target: self,
            config: None,
        }
    }

    pub(crate) fn with_config(self, config: Value) -> HomekitTargetConfig {
        HomekitTargetConfig {
            target: self,
            config: Some(config),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HomekitTargetConfig {
    pub(crate) target: HomekitTarget,
    pub(crate) config: Option<Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct HomekitEvent {
    pub(crate) target: HomekitTarget,
    pub(crate) value: serde_json::Value,
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
    BedroomDehumidifierFanSpeed(FanAirflow),
    LivingRoomHeatingState(HomekitHeatingState),
    BedroomHeatingState(HomekitHeatingState),
    KitchenHeatingState(HomekitHeatingState),
    RoomOfRequirementsHeatingState(HomekitHeatingState),
    BathroomHeatingState(HomekitHeatingState),
    LivingRoomBigHeatingDemand(Percent),
    LivingRoomSmallHeatingDemand(Percent),
    BedroomHeatingDemand(Percent),
    KitchenHeatingDemand(Percent),
    RoomOfRequirementsHeatingDemand(Percent),
    BathroomHeatingDemand(Percent),
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
    BedroomDehumidifierFanSpeed,
    LivingRoomHeatingState,
    BedroomHeatingState,
    KitchenHeatingState,
    RoomOfRequirementsHeatingState,
    BathroomHeatingState,
    LivingRoomBigHeatingDemand,
    LivingRoomSmallHeatingDemand,
    BedroomHeatingDemand,
    KitchenHeatingDemand,
    RoomOfRequirementsHeatingDemand,
    BathroomHeatingDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HomekitHeatingState {
    Off,
    Heat(DegreeCelsius),
    Auto,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Homekit {
    pub base_topic: String,
}

impl Homekit {
    pub async fn new_runner(
        &self,
        infrastructure: &mut Infrastructure,
        trigger_client: TriggerClient,
        state_change_rx: EventListener<HomeStateEvent>,
    ) -> HomekitRunner {
        let mqtt_receiver = infrastructure
            .mqtt_client
            .subscribe(format!("{}/from/set", &self.base_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        HomekitRunner::new(
            HomekitRegistry::default(),
            state_change_rx,
            infrastructure.mqtt_client.sender(&self.base_topic),
            mqtt_receiver,
            trigger_client,
        )
    }
}
