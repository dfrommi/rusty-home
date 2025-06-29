mod command;
mod event;

use api::state::{CurrentPowerUsage, Powered, TotalEnergyConsumption};

pub use command::TasmotaCommandExecutor;
pub use event::TasmotaIncomingDataSource;
use infrastructure::Mqtt;

use crate::core::DeviceConfig;

pub async fn new_incoming_data_source(
    base_topic: &str,
    config: &[(&str, TasmotaChannel)],
    mqtt: &mut Mqtt,
) -> TasmotaIncomingDataSource {
    let config = DeviceConfig::new(config);
    let tele_base_topic = format!("{}/tele", base_topic);
    let stat_base_topic = format!("{}/stat", base_topic);
    let rx = mqtt
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

#[derive(Debug, Clone)]
pub enum TasmotaChannel {
    EnergyMeter(CurrentPowerUsage, TotalEnergyConsumption),
    PowerToggle(Powered),
}

#[derive(Debug, Clone)]
pub enum TasmotaCommandTarget {
    PowerSwitch(&'static str),
}
