mod config;
mod incoming;

use crate::home::state::{CurrentPowerUsage, Opened, Presence, RelativeHumidity, Temperature, TotalEnergyConsumption};
use crate::home::trigger::RemoteTarget;
use incoming::Z2mIncomingDataSource;
use serde::Deserialize;

use crate::{
    Infrastructure,
    core::{DeviceConfig, process_incoming_data_source},
};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Zigbee2Mqtt {
    pub event_topic: String,
}

#[derive(Debug, Clone)]
enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption),
    PresenceFromLeakSensor(Presence),
    RemoteClick(RemoteTarget),
}

impl Zigbee2Mqtt {
    pub async fn new_incoming_data_processor(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl Future<Output = ()> + use<> {
        let ds = self.new_incoming_data_source(&mut infrastructure.mqtt_client).await;

        let api = infrastructure.api.clone();
        async move { process_incoming_data_source("Z2M", ds, &api).await }
    }

    async fn new_incoming_data_source(&self, mqtt: &mut infrastructure::Mqtt) -> Z2mIncomingDataSource {
        let config = DeviceConfig::new(&config::default_z2m_state_config());
        let rx = mqtt
            .subscribe(format!("{}/#", self.event_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        Z2mIncomingDataSource::new(self.event_topic.to_string(), config, rx)
    }
}
