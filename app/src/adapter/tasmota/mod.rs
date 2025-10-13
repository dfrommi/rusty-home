mod config;
mod incoming;
mod outgoing;

use crate::{
    adapter::command::CommandExecutor,
    home::state::{CurrentPowerUsage, Powered, TotalEnergyConsumption},
};

use incoming::TasmotaIncomingDataSource;
use outgoing::TasmotaCommandExecutor;
use serde::Deserialize;

use crate::{Infrastructure, core::DeviceConfig};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Tasmota {
    pub event_topic: String,
}

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    EnergyMeter(CurrentPowerUsage, TotalEnergyConsumption),
    PowerToggle(Powered),
}

#[derive(Debug, Clone)]
enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}

impl Tasmota {
    pub fn new_command_executor(&self, infrastructure: &Infrastructure) -> impl CommandExecutor + use<> {
        let tx = infrastructure.mqtt_client.new_publisher();
        let config = config::default_tasmota_command_config();
        TasmotaCommandExecutor::new(self.event_topic.clone(), config, tx)
    }

    pub async fn new_incoming_data_source(&self, infrastructure: &mut Infrastructure) -> TasmotaIncomingDataSource {
        let mqtt_client = &mut infrastructure.mqtt_client;
        let config = DeviceConfig::new(&config::default_tasmota_state_config());
        let tele_base_topic = format!("{}/tele", self.event_topic);
        let stat_base_topic = format!("{}/stat", self.event_topic);
        let rx = mqtt_client
            .subscribe_all(
                vec![
                    format!("{}/+/SENSOR", tele_base_topic),
                    format!("{}/+/POWER", stat_base_topic),
                ]
                .as_slice(),
            )
            .await
            .expect("Error subscribing to MQTT topic");

        TasmotaIncomingDataSource::new(tele_base_topic, stat_base_topic, config, rx)
    }
}
