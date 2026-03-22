mod accessory;
mod hap;
mod runtime;

use infrastructure::EventListener;
use serde::Deserialize;
use serde_json::Value;

use self::{
    accessory::HomekitRegistry,
    hap::{HomekitCharacteristic, HomekitService},
    runtime::HomekitRunner,
};
use crate::{Infrastructure, home_state::HomeStateEvent, trigger::TriggerClient};

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
pub struct Homekit {
    pub base_topic: String,
}

impl Homekit {
    #[allow(clippy::expect_used)]
    pub async fn new_runner(
        &self,
        infrastructure: &mut Infrastructure,
        trigger_client: TriggerClient,
        state_change_rx: EventListener<HomeStateEvent>,
    ) -> HomekitRunner {
        let mqtt_receiver = infrastructure
            .mqtt_client
            .subscribe(&self.base_topic, "from/set")
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
