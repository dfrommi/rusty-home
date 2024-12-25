mod adapter;
mod domain;

pub use adapter::HaMqttClient;
pub use adapter::HaRestClient;
use api::command::CommandTarget;
pub use domain::HaChannel;
pub use domain::HaServiceTarget;

use crate::core::CommandExecutor;
use crate::core::IncomingDataProcessor;

pub fn new_incoming_data_processor(
    client: HaRestClient,
    mqtt_client: HaMqttClient,
    config: &[(&str, HaChannel)],
) -> anyhow::Result<impl IncomingDataProcessor> {
    let collector = domain::HaIncomingDataProcessor::new(client, mqtt_client, config);
    Ok(collector)
}

pub fn new_command_executor(
    client: HaRestClient,
    config: &[(CommandTarget, HaServiceTarget)],
) -> impl CommandExecutor {
    domain::HaCommandExecutor::new(client, config)
}
