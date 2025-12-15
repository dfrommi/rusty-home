mod config;
mod incoming;

use crate::device_state::{CurrentPowerUsage, PowerAvailable, TotalEnergyConsumption};

use incoming::TasmotaIncomingDataSource;
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
    PowerToggle(PowerAvailable),
}

#[derive(Debug, Clone)]
enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}

impl Tasmota {
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
