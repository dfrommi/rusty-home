mod adapter;
mod domain;

use api::state::ChannelValue;
pub use domain::collect_states;
pub use domain::execute_commands;
pub use domain::CommandExecutor;
pub use domain::IncomingDataProcessor;
use support::DataPoint;

#[derive(Debug, Clone)]
pub enum IncomingData {
    StateValue(DataPoint<ChannelValue>),
}
