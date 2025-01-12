mod adapter;
mod domain;

pub use adapter::HaMqttClient;
pub use adapter::HaRestClient;
pub use domain::HaChannel;
pub use domain::HaServiceTarget;

use crate::core::IncomingDataProcessor;

pub use domain::HaCommandExecutor;

pub fn new_incoming_data_processor(
    client: HaRestClient,
    mqtt_client: HaMqttClient,
    config: &[(&str, HaChannel)],
) -> anyhow::Result<impl IncomingDataProcessor> {
    let collector = domain::HaIncomingDataProcessor::new(client, mqtt_client, config);
    Ok(collector)
}
