use api::DbEventListener;
use support::time::Duration;
use tokio::sync::broadcast;

use crate::Database;

#[derive(Debug, Clone)]
pub struct StateChangedEvent;

#[derive(Debug, Clone)]
pub struct UserTriggerEvent;

pub struct AppEventListener {
    database: Database,
    db_listener: DbEventListener,

    state_changed_tx: tokio::sync::broadcast::Sender<StateChangedEvent>,
    user_trigger_tx: tokio::sync::broadcast::Sender<UserTriggerEvent>,
}

impl AppEventListener {
    pub fn new(db_listener: DbEventListener, database: Database) -> Self {
        Self {
            db_listener,
            database,
            state_changed_tx: broadcast::channel(128).0,
            user_trigger_tx: broadcast::channel(16).0,
        }
    }

    pub fn new_state_changed_listener(&self) -> broadcast::Receiver<StateChangedEvent> {
        self.state_changed_tx.subscribe()
    }

    pub fn new_user_trigger_event_listener(&self) -> broadcast::Receiver<UserTriggerEvent> {
        self.user_trigger_tx.subscribe()
    }

    //consume as much as possible before triggering app event to debounce planning etc
    pub async fn dispatch_events(mut self) -> anyhow::Result<()> {
        self.db_listener.start_listening().await?;
        self.database.preload_ts_cache().await?;

        loop {
            let events = match self.db_listener.recv_multi(Duration::millis(5)).await {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!("Error receiving database event: {:?}", e);
                    continue;
                }
            };

            let mut state_changed = false;
            let mut user_trigger = false;

            for event in events {
                match event {
                    api::DbEvent::StateValueAdded { tag_id, .. } => {
                        self.database.invalidate_ts_cache(tag_id).await;
                        state_changed = true;
                    }
                    api::DbEvent::UserTriggerInsert { .. } => {
                        user_trigger = true;
                    }

                    //TODO invalidate command cache, but target is not easily available
                    _ => {}
                }
            }

            if state_changed {
                if let Err(e) = self.state_changed_tx.send(StateChangedEvent) {
                    tracing::error!("Error sending state changed event: {:?}", e);
                }
            }

            if user_trigger {
                if let Err(e) = self.user_trigger_tx.send(UserTriggerEvent) {
                    tracing::error!("Error sending user trigger event: {:?}", e);
                }
            }
        }
    }
}
