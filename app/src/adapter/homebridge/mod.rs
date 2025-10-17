mod accessory;
mod hap;
mod runtime;

use serde::Deserialize;
use serde_json::Value;
use tokio::sync::broadcast::Receiver;

use crate::{
    Infrastructure,
    adapter::homebridge::{
        accessory::HomekitRegistry,
        hap::{HomekitCharacteristic, HomekitService},
        runtime::HomebridgeRunner,
    },
    core::timeseries::DataPoint,
    home::state::HomeStateValue,
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

#[derive(Clone, Deserialize, Debug)]
pub struct Homebridge {
    pub base_topic: String,
}

impl Homebridge {
    pub async fn new_runner(
        &self,
        infrastructure: &mut Infrastructure,
        state_change_rx: Receiver<DataPoint<HomeStateValue>>,
    ) -> HomebridgeRunner {
        let mqtt_receiver = infrastructure
            .mqtt_client
            .subscribe(format!("{}/from/set", &self.base_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        HomebridgeRunner::new(
            HomekitRegistry::default(),
            state_change_rx,
            infrastructure.mqtt_client.new_publisher(),
            mqtt_receiver,
            self.base_topic.clone(),
            infrastructure.api.clone(),
        )
    }
}
