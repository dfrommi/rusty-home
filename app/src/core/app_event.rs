use crate::core::time::Duration;
use tokio::sync::broadcast;

use crate::core::HomeApi;

use super::persistence::listener::{DbEvent, DbEventListener};

#[derive(Debug, Clone)]
pub struct StateChangedEvent;

#[derive(Debug, Clone)]
pub struct UserTriggerEvent;

#[derive(Debug, Clone)]
pub struct CommandAddedEvent;

#[derive(Debug, Clone)]
pub struct EnergyReadingAddedEvent {
    pub id: i64,
}

pub struct AppEventListener {
    api: HomeApi,
    db_listener: DbEventListener,

    state_changed_tx: tokio::sync::broadcast::Sender<StateChangedEvent>,
    user_trigger_tx: tokio::sync::broadcast::Sender<UserTriggerEvent>,
    command_added_tx: tokio::sync::broadcast::Sender<CommandAddedEvent>,
    energy_reading_added_tx: tokio::sync::broadcast::Sender<EnergyReadingAddedEvent>,
}

impl AppEventListener {
    pub fn new(db_listener: DbEventListener, api: HomeApi) -> Self {
        Self {
            db_listener,
            api,
            state_changed_tx: broadcast::channel(128).0,
            user_trigger_tx: broadcast::channel(16).0,
            command_added_tx: broadcast::channel(16).0,
            energy_reading_added_tx: broadcast::channel(16).0,
        }
    }

    pub fn new_state_changed_listener(&self) -> broadcast::Receiver<StateChangedEvent> {
        self.state_changed_tx.subscribe()
    }

    pub fn new_user_trigger_event_listener(&self) -> broadcast::Receiver<UserTriggerEvent> {
        self.user_trigger_tx.subscribe()
    }

    pub fn new_command_added_listener(&self) -> broadcast::Receiver<CommandAddedEvent> {
        self.command_added_tx.subscribe()
    }

    pub fn new_energy_reading_added_listener(&self) -> broadcast::Receiver<EnergyReadingAddedEvent> {
        self.energy_reading_added_tx.subscribe()
    }

    //consume as much as possible before triggering app event to debounce planning etc
    pub async fn dispatch_events(mut self) -> anyhow::Result<()> {
        self.db_listener.start_listening().await?;

        loop {
            let events = match self.db_listener.recv_multi(Duration::millis(5)).await {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!("Error receiving database event: {:?}", e);
                    continue;
                }
            };

            //only emit once if event is not bound to a specific item/row
            let mut state_changed_sent = false;
            let mut user_trigger_sent = false;
            let mut command_added_sent = false;

            for event in events {
                match event {
                    DbEvent::StateValueAdded { tag_id, .. } if !state_changed_sent => {
                        self.api.invalidate_ts_cache(tag_id).await;
                        if let Err(e) = self.state_changed_tx.send(StateChangedEvent) {
                            tracing::error!("Error sending state changed event: {:?}", e);
                        }
                        state_changed_sent = true;
                    }
                    DbEvent::UserTriggerInsert { id } if !user_trigger_sent => {
                        if let Err(e) = self.api.invalidate_user_trigger_cache_by_id(&id).await {
                            tracing::error!("Error invalidating user trigger cache for database event {}: {:?}", id, e);
                        }
                        if let Err(e) = self.user_trigger_tx.send(UserTriggerEvent) {
                            tracing::error!("Error sending user trigger event: {:?}", e);
                        }
                        user_trigger_sent = true;
                    }

                    DbEvent::CommandAdded { .. } if !command_added_sent => {
                        if let Err(e) = self.command_added_tx.send(CommandAddedEvent) {
                            tracing::error!("Error sending command added event: {:?}", e);
                        }
                        command_added_sent = true;
                    }

                    DbEvent::EnergyReadingInsert { id } => {
                        if let Err(e) = self.energy_reading_added_tx.send(EnergyReadingAddedEvent { id }) {
                            tracing::error!("Error sending energy reading added event: {:?}", e);
                        }
                    }

                    //TODO invalidate command cache, but target is not easily available
                    _ => {}
                }
            }
        }
    }
}
