use api::DbEventListener;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct CommandAddedEvent;

#[derive(Debug, Clone)]
pub struct EnergyReadingAddedEvent {
    pub id: i64,
}

pub struct AppEventListener {
    db_listener: DbEventListener,

    command_added_tx: tokio::sync::broadcast::Sender<CommandAddedEvent>,
    energy_reading_added_tx: tokio::sync::broadcast::Sender<EnergyReadingAddedEvent>,
}

impl AppEventListener {
    pub fn new(db_listener: DbEventListener) -> Self {
        Self {
            db_listener,
            command_added_tx: broadcast::channel(16).0,
            energy_reading_added_tx: broadcast::channel(16).0,
        }
    }

    pub fn new_command_added_listener(&self) -> broadcast::Receiver<CommandAddedEvent> {
        self.command_added_tx.subscribe()
    }

    pub fn new_energy_reading_added_listener(
        &self,
    ) -> broadcast::Receiver<EnergyReadingAddedEvent> {
        self.energy_reading_added_tx.subscribe()
    }

    pub async fn dispatch_events(mut self) -> anyhow::Result<()> {
        self.db_listener.start_listening().await?;

        loop {
            match self.db_listener.recv().await {
                Ok(api::DbEvent::CommandAdded { .. }) => {
                    if let Err(e) = self.command_added_tx.send(CommandAddedEvent) {
                        tracing::error!("Error sending command added event: {:?}", e);
                    }
                }

                Ok(api::DbEvent::EnergyReadingInsert { id }) => {
                    if let Err(e) = self
                        .energy_reading_added_tx
                        .send(EnergyReadingAddedEvent { id })
                    {
                        tracing::error!("Error sending energy reading added event: {:?}", e);
                    }
                }

                Ok(_) => {}

                Err(e) => {
                    tracing::error!("Error receiving database event: {:?}", e);
                }
            }
        }
    }
}
