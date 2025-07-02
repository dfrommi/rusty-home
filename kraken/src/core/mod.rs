mod adapter;
mod domain;
pub mod event;

pub use adapter::DeviceConfig;
pub use adapter::IncomingDataSource;
pub use adapter::process_incoming_data_source;
use api::state::ChannelValue;
use api::trigger::UserTrigger;
pub use domain::CommandExecutor;
pub use domain::execute_commands;

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
