mod adapter;
mod domain;
pub mod event;

pub use adapter::DeviceConfig;
pub use adapter::IncomingDataSource;
pub use adapter::IncomingMqttDataProcessor;
pub use adapter::process_incoming_data_source;
use api::state::ChannelValue;
use api::trigger::UserTrigger;
pub use domain::CommandExecutor;
pub use domain::IncomingDataProcessor;
pub use domain::collect_states;
pub use domain::execute_commands;

use infrastructure::MqttInMessage;
use support::DataPoint;
use support::time::DateTime;

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<ChannelValue>),
    UserTrigger(UserTrigger),
    ItemAvailability(ItemAvailability),
}

#[derive(Debug, Clone)]
pub struct ItemAvailability {
    pub source: String,
    pub item: String,
    pub last_seen: DateTime,
    pub marked_offline: bool,
}

pub trait IncomingMqttEventParser<C> {
    fn topic_patterns(&self) -> Vec<String>;
    fn device_id(&self, msg: &MqttInMessage) -> Option<String>;
    fn get_events(
        &self,
        device_id: &str,
        channel: &C,
        msg: &MqttInMessage,
    ) -> anyhow::Result<Vec<IncomingData>>;
}
