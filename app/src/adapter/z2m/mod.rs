mod config;
mod incoming;
mod outgoing;

use crate::adapter::command::CommandExecutor;
use crate::automation::Thermostat;
use crate::core::unit::KiloWattHours;
use crate::device_state::{
    CurrentPowerUsage, HeatingDemand, Opened, RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};
use incoming::Z2mIncomingDataSource;
use outgoing::Z2mCommandExecutor;
use serde::Deserialize;

use crate::{Infrastructure, core::DeviceConfig};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Zigbee2Mqtt {
    pub event_topic: String,
}

#[derive(Debug, Clone)]
pub enum Z2mChannel {
    ClimateSensor(Temperature, RelativeHumidity),
    ContactSensor(Opened),
    PowerPlug(CurrentPowerUsage, TotalEnergyConsumption, KiloWattHours),
    Thermostat(Thermostat, SetPoint, HeatingDemand, Opened),
}

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    Thermostat(&'static str),
}

impl Zigbee2Mqtt {
    pub fn new_command_executor(&self, infrastructure: &Infrastructure) -> impl CommandExecutor + use<> {
        self.new_z2m_command_executor(&infrastructure.mqtt_client)
    }

    pub async fn new_incoming_data_source(&self, infrastructure: &mut Infrastructure) -> Z2mIncomingDataSource {
        let mqtt = &mut infrastructure.mqtt_client;
        let config = DeviceConfig::new(&config::default_z2m_state_config());
        let rx = mqtt
            .subscribe(format!("{}/#", self.event_topic))
            .await
            .expect("Error subscribing to MQTT topic");

        Z2mIncomingDataSource::new(self.event_topic.to_string(), config, rx)
    }

    fn new_z2m_command_executor(&self, mqtt: &infrastructure::Mqtt) -> Z2mCommandExecutor {
        let tx = mqtt.new_publisher();
        let config = config::default_z2m_command_config();
        Z2mCommandExecutor::new(self.event_topic.clone(), config, tx)
    }
}
