use crate::{core::StateCollector, Database};
use axum::Router;
use domain::{EnergyMeterService, EnergyMeterStateCollector};

mod adapter;
mod domain;

pub fn new(db: Database) -> anyhow::Result<(impl StateCollector, Router)> {
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let service = EnergyMeterService::new(db.clone(), tx);
    let router = adapter::http::router(service);

    let collector = EnergyMeterStateCollector::new(db.clone(), rx);

    Ok((collector, router))
}
