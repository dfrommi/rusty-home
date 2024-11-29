use crate::{core::StateCollector, Database};
use api::EnergyReadingInsertEvent;
use axum::Router;
use domain::{EnergyMeterService, EnergyMeterStateCollector};
use tokio::sync::broadcast::Receiver;

mod adapter;
mod domain;

pub fn new(
    db: Database,
    new_reading_rx: Receiver<EnergyReadingInsertEvent>,
) -> anyhow::Result<(impl StateCollector, Router)> {
    let service = EnergyMeterService::new(db.clone());
    let router = adapter::http::router(service);

    let collector = EnergyMeterStateCollector::new(db.clone(), new_reading_rx);

    Ok((collector, router))
}
