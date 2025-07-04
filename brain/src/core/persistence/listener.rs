use serde::Deserialize;
use sqlx::postgres::{PgListener, PgNotification};
use support::time::Duration;

const THING_VALUE_ADDED_EVENT: &str = "thing_values_insert";
const THING_COMMAND_ADDED_EVENT: &str = "thing_command_insert";
const ENERGY_READING_INSERT_EVENT: &str = "energy_reading_insert";
const USER_TRIGGER_INSERT_EVENT: &str = "user_trigger_insert";

#[derive(Debug)]
pub struct DbEventListener {
    db_listener: PgListener,
    listening: bool,
}

#[derive(Debug, Clone)]
pub enum DbEvent {
    StateValueAdded { id: i64, tag_id: i64 },
    CommandAdded { id: i64 },
    EnergyReadingInsert { id: i64 },
    UserTriggerInsert { id: i64 },
}

#[derive(Debug, Clone, Deserialize)]
struct StateValueAddedPayload {
    pub id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct CommandAddedPayload {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct EnergyReadingInsertPayload {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct UserTriggerInsertPayload {
    pub id: i64,
}

impl DbEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        Self {
            db_listener,
            listening: false,
        }
    }

    pub async fn start_listening(&mut self) -> anyhow::Result<()> {
        tracing::debug!("Start listening for DB-events");

        self.db_listener
            .listen_all(vec![
                THING_VALUE_ADDED_EVENT,
                THING_COMMAND_ADDED_EVENT,
                ENERGY_READING_INSERT_EVENT,
                USER_TRIGGER_INSERT_EVENT,
            ])
            .await?;
        self.listening = true;
        Ok(())
    }

    pub async fn recv_multi(&mut self, timeout: Duration) -> anyhow::Result<Vec<DbEvent>> {
        if !self.listening {
            return Err(anyhow::anyhow!("Event listener not started"));
        }

        let mut events = vec![self.recv().await?];

        let timeout: std::time::Duration = timeout.into();
        while let Ok(Ok(event)) = tokio::time::timeout(timeout, self.recv()).await {
            events.push(event);
        }

        Ok(events)
    }

    pub async fn recv(&mut self) -> anyhow::Result<DbEvent> {
        if !self.listening {
            return Err(anyhow::anyhow!("Event listener not started"));
        }

        let event = self.db_listener.recv().await?;
        tracing::trace!("Received DB-event: {:?}", event);

        event.try_into()
    }
}

impl TryInto<DbEvent> for PgNotification {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<DbEvent, Self::Error> {
        match self.channel() {
            THING_VALUE_ADDED_EVENT => {
                let payload: StateValueAddedPayload = serde_json::from_str(self.payload())?;
                Ok(DbEvent::StateValueAdded {
                    id: payload.id,
                    tag_id: payload.tag_id,
                })
            }
            THING_COMMAND_ADDED_EVENT => {
                let payload: CommandAddedPayload = serde_json::from_str(self.payload())?;
                Ok(DbEvent::CommandAdded { id: payload.id })
            }
            ENERGY_READING_INSERT_EVENT => {
                let payload: EnergyReadingInsertPayload = serde_json::from_str(self.payload())?;
                Ok(DbEvent::EnergyReadingInsert { id: payload.id })
            }
            USER_TRIGGER_INSERT_EVENT => {
                let payload: UserTriggerInsertPayload = serde_json::from_str(self.payload())?;
                Ok(DbEvent::UserTriggerInsert { id: payload.id })
            }
            _ => Err(anyhow::anyhow!("Unknown event channel: {}", self.channel())),
        }
    }
}
