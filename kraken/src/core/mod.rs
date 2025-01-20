mod adapter;
mod domain;
pub mod event;

pub use adapter::IncomingMqttDataProcessor;
use api::state::ChannelValue;
use api::trigger::UserTrigger;
pub use domain::collect_states;
pub use domain::execute_commands;
pub use domain::CommandExecutor;
pub use domain::IncomingDataProcessor;

use infrastructure::mqtt::MqttInMessage;
use support::time::DateTime;
use support::DataPoint;

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
