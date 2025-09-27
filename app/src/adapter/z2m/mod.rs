mod config;
mod incoming;
mod outgoing;

use crate::core::CommandExecutor;
use crate::core::unit::KiloWattHours;
use crate::home::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Presence, RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};
use crate::home::trigger::RemoteTarget;
use incoming::Z2mIncomingDataSource;
use outgoing::Z2mCommandExecutor;
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
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption, KiloWattHours),
    PresenceFromLeakSensor(Presence),
    RemoteClick(RemoteTarget),
    Thermostat(SetPoint, HeatingDemand, Opened),
}

#[derive(Debug, Clone)]
enum Z2mCommandTarget {
    Thermostat(&'static str),
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

    pub fn new_command_executor(&self, infrastructure: &Infrastructure) -> impl CommandExecutor + use<> {
        let tx = infrastructure.mqtt_client.new_publisher();
        let config = config::default_z2m_command_config();
        Z2mCommandExecutor::new(self.event_topic.clone(), config, tx)
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
