mod adapter;
mod domain;

pub use adapter::HaMqttClient;
pub use adapter::HaRestClient;
use api::command::CommandTarget;
pub use domain::HaChannel;
pub use domain::HaServiceTarget;

use crate::core::CommandExecutor;
use crate::core::StateCollector;

pub fn new_state_collector(
    client: HaRestClient,
    mqtt_client: HaMqttClient,
    config: &[(&str, HaChannel)],
) -> anyhow::Result<impl StateCollector> {
    let collector = domain::HaStateCollector::new(client, mqtt_client, config);
    Ok(collector)
}

pub fn new_command_executor(
    client: HaRestClient,
    config: &[(CommandTarget, HaServiceTarget)],
) -> impl CommandExecutor {
    domain::HaCommandExecutor::new(client, config)
}
