mod accessory;
mod hap;
mod runtime;

use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast::Receiver;

use self::{
    accessory::HomekitRegistry,
    hap::{HomekitCharacteristic, HomekitService},
    runtime::HomekitRunner,
};
use crate::{
    Infrastructure,
    core::timeseries::DataPoint,
    core::unit::DegreeCelsius,
    home::state::{FanAirflow, HomeStateValue},
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

#[derive(Clone, Deserialize, Debug)]
pub struct Homekit {
    pub base_topic: String,
}

impl Homekit {
    pub async fn new_runner(
        &self,
        infrastructure: &mut Infrastructure,
        state_change_rx: Receiver<DataPoint<HomeStateValue>>,
    ) -> HomekitRunner {
        let mqtt_receiver = infrastructure
            .mqtt_client
            .subscribe(format!("{}/from/set", &self.base_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        HomekitRunner::new(
            HomekitRegistry::default(),
            state_change_rx,
            infrastructure.mqtt_client.new_publisher(),
            mqtt_receiver,
            self.base_topic.clone(),
            infrastructure.api.clone(),
        )
    }
}
