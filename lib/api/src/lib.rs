pub mod command;
pub mod state;

use std::collections::HashMap;

use anyhow::{bail, Result};

use sqlx::postgres::PgListener;
pub use state::db::get_tag_id;
use tokio::sync::broadcast::{Receiver, Sender};

pub const THING_VALUE_ADDED_EVENT: &str = "thing_values_insert";
pub const THING_COMMAND_ADDED_EVENT: &str = "thing_command_insert";

#[derive(Debug)]
pub struct DbEventListener {
    db_listener: PgListener,
    sender_by_topic: HashMap<String, Sender<()>>,
}

impl DbEventListener {
    pub fn new(db_listener: PgListener, topics: Vec<&str>) -> Self {
        let mut sender_by_topic = HashMap::new();

        for topic in topics {
            let (tx, _) = tokio::sync::broadcast::channel(1);
            sender_by_topic.insert(topic.to_string(), tx);
        }

        Self {
            db_listener,
            sender_by_topic,
        }
    }

    pub fn new_listener(&self, topic: &str) -> Result<Receiver<()>> {
        match self.sender_by_topic.get(topic) {
            Some(tx) => Ok(tx.subscribe()),
            None => bail!("Unknown topic {}", topic),
        }
    }

    pub async fn dispatch_events(mut self) -> Result<()> {
        let topics: Vec<&str> = self.sender_by_topic.keys().map(|s| s.as_str()).collect();
        self.db_listener.listen_all(topics).await?;

        loop {
            match self.db_listener.recv().await {
                Ok(notification) => {
                    let topic = notification.channel();
                    match self.sender_by_topic.get(topic) {
                        Some(tx) => {
                            if let Err(e) = tx.send(()) {
                                tracing::error!(
                                    "Error dispatching event for topic {}: {}",
                                    topic,
                                    e
                                );
                            }
                        }
                        None => {
                            tracing::warn!("Received notification on unknown topic: {}", topic);
                        }
                    }
                }
                Err(e) => tracing::error!("Error receiving notification: {}", e),
            }
        }
    }
}
