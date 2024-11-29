pub mod command;
pub mod state;

use anyhow::Result;

use serde::Deserialize;
use sqlx::postgres::{PgListener, PgNotification};
pub use state::db::get_tag_id;
use tokio::sync::broadcast::{Receiver, Sender};

const THING_VALUE_ADDED_EVENT: &str = "thing_values_insert";
const THING_COMMAND_ADDED_EVENT: &str = "thing_command_insert";
const ENERGY_READING_INSERT_EVENT: &str = "energy_reading_insert";

#[derive(Debug)]
pub struct DbEventListener {
    db_listener: PgListener,
    thing_value_added_tx: Sender<StateValueAddedEvent>,
    thing_command_added_tx: Sender<CommandAddedEvent>,
    energy_reading_insert_tx: Sender<EnergyReadingInsertEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateValueAddedEvent {
    pub id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandAddedEvent {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnergyReadingInsertEvent {
    pub id: i64,
}

impl DbEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        let (thing_value_added_tx, _) = tokio::sync::broadcast::channel(16);
        let (thing_command_added_tx, _) = tokio::sync::broadcast::channel(16);
        let (energy_reading_insert_tx, _) = tokio::sync::broadcast::channel(16);

        Self {
            db_listener,
            thing_value_added_tx,
            thing_command_added_tx,
            energy_reading_insert_tx,
        }
    }

    pub fn new_state_value_added_listener(&self) -> Receiver<StateValueAddedEvent> {
        self.thing_value_added_tx.subscribe()
    }

    pub fn new_command_added_listener(&self) -> Receiver<CommandAddedEvent> {
        self.thing_command_added_tx.subscribe()
    }

    pub fn new_energy_reading_insert_listener(&self) -> Receiver<EnergyReadingInsertEvent> {
        self.energy_reading_insert_tx.subscribe()
    }

    pub async fn dispatch_events(mut self) -> Result<()> {
        let mut topics = vec![];
        if self.thing_value_added_tx.receiver_count() > 0 {
            topics.push(THING_VALUE_ADDED_EVENT);
        }

        if self.thing_command_added_tx.receiver_count() > 0 {
            topics.push(THING_COMMAND_ADDED_EVENT);
        }

        if self.energy_reading_insert_tx.receiver_count() > 0 {
            topics.push(ENERGY_READING_INSERT_EVENT);
        }

        if topics.is_empty() {
            tracing::warn!("No events were subscribed");
            return Ok(());
        }

        self.db_listener.listen_all(topics).await?;

        loop {
            match self.db_listener.recv().await {
                Ok(notification) => match notification.channel() {
                    THING_VALUE_ADDED_EVENT => {
                        self.forward_event(&self.thing_value_added_tx, &notification);
                    }
                    THING_COMMAND_ADDED_EVENT => {
                        self.forward_event(&self.thing_command_added_tx, &notification);
                    }
                    ENERGY_READING_INSERT_EVENT => {
                        self.forward_event(&self.energy_reading_insert_tx, &notification);
                    }
                    topic => {
                        tracing::warn!("Received unsupported event for topic {}", topic);
                    }
                },
                Err(e) => tracing::error!("Error receiving notification: {}", e),
            }
        }
    }

    fn forward_event<T: serde::de::DeserializeOwned>(
        &self,
        tx: &Sender<T>,
        notification: &PgNotification,
    ) {
        let event = serde_json::from_str::<T>(notification.payload());
        if let Err(e) = &event {
            tracing::error!("Error deserializing event {:?}: {}", notification, e);
            return;
        }

        if let Err(e) = tx.send(event.unwrap()) {
            tracing::error!("Error sending event {:?}: {}", notification, e);
        }
    }
}
