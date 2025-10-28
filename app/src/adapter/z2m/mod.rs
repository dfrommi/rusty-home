mod config;
mod incoming;
mod outgoing;

use std::sync::Arc;

use crate::adapter::command::CommandExecutor;
use crate::adapter::z2m::incoming::ThermostatGroup;
use crate::core::unit::KiloWattHours;
use crate::home::Thermostat;
use crate::home::state::{
    CurrentPowerUsage, HeatingDemand, Opened, Presence, RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
};
use crate::home::trigger::RemoteTarget;
use incoming::Z2mIncomingDataSource;
use outgoing::Z2mCommandExecutor;
use serde::Deserialize;
use tokio::sync::Mutex;

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
    PresenceFromLeakSensor(Presence),
    RemoteClick(RemoteTarget),
    Thermostat(Thermostat, SetPoint, HeatingDemand, Opened, Option<Arc<Mutex<ThermostatGroup>>>),
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

        Z2mIncomingDataSource::new(self.event_topic.to_string(), config, rx, self.new_z2m_command_executor(mqtt))
    }

    fn new_z2m_command_executor(&self, mqtt: &infrastructure::Mqtt) -> Z2mCommandExecutor {
        let tx = mqtt.new_publisher();
        let config = config::default_z2m_command_config();
        Z2mCommandExecutor::new(self.event_topic.clone(), config, tx)
    }
}
