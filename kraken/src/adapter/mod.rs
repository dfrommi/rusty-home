use chrono::{DateTime, Utc};

use api::{state::ChannelValue, BackendApi};

mod homeassistant;

pub use homeassistant::process_ha_commands;
pub use homeassistant::process_ha_events;

pub enum IncomingMessage {
    HomeAssistant { payload: String },
}

pub enum OutgoingMessage {
    HomeAssistant { payload: String },
}

#[derive(Debug, Clone)]
pub struct PersistentDataPoint {
    value: ChannelValue,
    timestamp: DateTime<Utc>,
}
