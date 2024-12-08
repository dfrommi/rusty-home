use crate::{core::StateCollector, Database};
use api::EnergyReadingInsertEvent;
use domain::{EnergyMeterService, EnergyMeterStateCollector};
use tokio::sync::broadcast::Receiver;

mod adapter;
mod domain;

pub fn new(
    db: Database,
    new_reading_rx: Receiver<EnergyReadingInsertEvent>,
) -> anyhow::Result<impl StateCollector> {
    let collector = EnergyMeterStateCollector::new(db.clone(), new_reading_rx);
    Ok(collector)
}

pub fn new_web_service(db: Database) -> actix_web::Scope {
    let service = EnergyMeterService::new(db);
    adapter::http::new_actix_web_scope(service)
}
