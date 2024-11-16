mod command;
mod plan_log;
mod state;

use anyhow::Result;
use api::EventListener;
use sqlx::postgres::PgListener;
use tokio::sync::broadcast::Receiver;

#[derive(Debug)]
pub struct HomeEventListener {
    delegate: EventListener,
}

impl HomeEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        Self {
            delegate: EventListener::new(db_listener, vec![api::THING_VALUE_ADDED_EVENT]),
        }
    }

    pub fn new_thing_value_added_listener(&self) -> Receiver<()> {
        self.delegate
            .new_listener(api::THING_VALUE_ADDED_EVENT)
            .unwrap()
    }

    pub async fn dispatch_events(self) -> Result<()> {
        self.delegate.dispatch_events().await
    }
}
