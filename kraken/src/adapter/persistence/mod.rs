mod command;
mod energy_reading;
mod state;

use api::EventListener;
use sqlx::{postgres::PgListener, PgPool};
use tokio::sync::broadcast::Receiver;

use anyhow::Result;

pub use command::CommandRepository;
#[allow(unused_imports)]
pub use energy_reading::EnergyReadingRepository;
pub use state::StateRepository;

#[derive(Debug)]
pub struct BackendEventListener {
    delegate: EventListener,
}

impl BackendEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        Self {
            delegate: EventListener::new(db_listener, vec![api::THING_COMMAND_ADDED_EVENT]),
        }
    }

    pub fn new_command_added_listener(&self) -> Receiver<()> {
        self.delegate
            .new_listener(api::THING_COMMAND_ADDED_EVENT)
            .unwrap()
    }

    pub async fn dispatch_events(self) -> Result<()> {
        self.delegate.dispatch_events().await
    }
}

#[derive(Debug, Clone)]
pub struct BackendApi {
    db_pool: PgPool,
}

impl BackendApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}
