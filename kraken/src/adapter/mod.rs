use chrono::{DateTime, Utc};

use api::state::ChannelValue;

mod homeassistant;
pub mod persistence;

pub use homeassistant::process_ha_commands;
pub use homeassistant::process_ha_events;

#[derive(Debug, Clone)]
pub struct PersistentDataPoint {
    value: ChannelValue,
    timestamp: DateTime<Utc>,
}
