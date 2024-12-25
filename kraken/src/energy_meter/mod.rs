use crate::{core::IncomingDataProcessor, Database};
use api::EnergyReadingInsertEvent;
use domain::{EnergyMeterIncomingDataProcessor, EnergyMeterService};
use tokio::sync::broadcast::Receiver;

mod adapter;
mod domain;

pub fn new(
    db: Database,
    new_reading_rx: Receiver<EnergyReadingInsertEvent>,
) -> impl IncomingDataProcessor {
    EnergyMeterIncomingDataProcessor::new(db.clone(), new_reading_rx)
}

pub fn new_web_service(db: Database) -> actix_web::Scope {
    let service = EnergyMeterService::new(db);
    adapter::http::new_actix_web_scope(service)
}
