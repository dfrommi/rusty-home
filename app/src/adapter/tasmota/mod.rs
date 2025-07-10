mod config;
mod incoming;
mod outgoing;

use crate::home::state::{CurrentPowerUsage, Powered, TotalEnergyConsumption};

use incoming::TasmotaIncomingDataSource;
use outgoing::TasmotaCommandExecutor;
use serde::Deserialize;

use crate::{
    Infrastructure,
    core::{CommandExecutor, DeviceConfig, process_incoming_data_source},
};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Tasmota {
    pub event_topic: String,
}

#[derive(Debug, Clone)]
enum TasmotaChannel {
    EnergyMeter(CurrentPowerUsage, TotalEnergyConsumption),
    PowerToggle(Powered),
}

#[derive(Debug, Clone)]
enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}

impl Tasmota {
    pub async fn new_incoming_data_processor(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl Future<Output = ()> + use<> {
        let ds = self.new_incoming_data_source(&mut infrastructure.mqtt_client).await;

        let api = infrastructure.api.clone();
        async move { process_incoming_data_source("Tasmota", ds, &api).await }
    }

    pub fn new_command_executor(&self, infrastructure: &Infrastructure) -> impl CommandExecutor + use<> {
        let tx = infrastructure.mqtt_client.new_publisher();
        let config = config::default_tasmota_command_config();
        TasmotaCommandExecutor::new(self.event_topic.clone(), config, tx)
    }

    async fn new_incoming_data_source(&self, mqtt_client: &mut infrastructure::Mqtt) -> TasmotaIncomingDataSource {
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
