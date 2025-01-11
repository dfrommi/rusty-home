mod adapter;
mod domain;
pub mod event;

use api::state::ChannelValue;
use api::trigger::UserTrigger;
pub use domain::collect_states;
pub use domain::execute_commands;
pub use domain::CommandExecutor;
pub use domain::IncomingDataProcessor;
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
